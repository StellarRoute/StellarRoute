import * as freighterModule from '@stellar/freighter-api';
import {
  isConnected as isLobstrConnected,
  getPublicKey as getLobstrPublicKey,
  signTransaction as signLobstrTransaction,
} from '@lobstrco/signer-extension-api';

import type {
  AvailableWallet,
  SupportedWallet,
  WalletSession,
  Capabilities,
  Capability,
  CapabilityStatus,
} from './types';
import { resolveFreighterApi, type FreighterApi } from './freighter-api';
import { normalizeAppNetwork } from '@/lib/network-policy';

let freighterApiCache: FreighterApi | null = null;

function freighterApi(): FreighterApi {
  if (!freighterApiCache) {
    freighterApiCache = resolveFreighterApi(freighterModule);
  }
  return freighterApiCache;
}

/** Freighter reports `TESTNET` / `PUBLIC`; app policy uses lowercase `testnet` / `mainnet`. */
function normalizeFreighterNetwork(network: string): string {
  return normalizeAppNetwork(network) ?? network.trim().toLowerCase();
}

type AlbedoClient = {
  publicKey: () => Promise<{ pubkey?: string; publicKey?: string }>;
  tx: (opts: { xdr: string; network?: string; pubkey?: string }) => Promise<{
    signed_envelope_xdr?: string;
    signedXdr?: string;
    xdr?: string;
  }>;
};

declare global {
  interface Window {
    albedo?: AlbedoClient;
  }
}

const ALBEDO_INTENT_SCRIPT_URL =
  'https://unpkg.com/@albedo-link/intent/lib/albedo.intent.js';

let albedoScriptPromise: Promise<AlbedoClient> | null = null;

function getWindowRecord(): Record<string, unknown> | null {
  return typeof window === 'undefined'
    ? null
    : (window as unknown as Record<string, unknown>);
}

type XBullClient = {
  connect: () => Promise<{ publicKey: string }>;
  sign?: (opts: {
    xdr: string;
    network?: string;
    publicKey?: string;
  }) => Promise<string>;
  getNetwork?: () => Promise<
    | { network: string; networkPassphrase?: string; error?: undefined }
    | { error: { message?: string; code?: number }; network?: undefined }
  >;
};

/** Resolve the injected xBull client (`window.xbull` or SEP-43 `window.xBullSDK`). */
function getXBullClient(): XBullClient | undefined {
  const win = getWindowRecord();
  if (!win) return undefined;
  const client = (win.xbull ?? win.xBullSDK) as XBullClient | undefined;
  return client?.connect ? client : undefined;
}

/**
 * Map xBull network identifiers (PUBLIC/TESTNET, public/testnet, passphrases)
 * onto Freighter-compatible session network strings.
 */
function normalizeXbullNetworkName(
  network: string,
  networkPassphrase?: string
): string {
  const key = network.trim().toLowerCase();
  if (
    key === 'public' ||
    key === 'pubnet' ||
    key === 'mainnet' ||
    key === 'production'
  ) {
    return 'public';
  }
  if (key === 'testnet' || key === 'test') {
    return 'testnet';
  }
  if (networkPassphrase?.includes('Public Global Stellar Network')) {
    return 'public';
  }
  if (networkPassphrase?.includes('Test SDF Network')) {
    return 'testnet';
  }
  return key;
}

/**
 * Read the wallet's currently selected network via getNetwork when available.
 * Falls back to the app-configured network only when the API is missing.
 */
async function resolveXbullNetwork(xbull: XBullClient): Promise<string> {
  if (typeof xbull.getNetwork === 'function') {
    try {
      const res = await xbull.getNetwork();
      if (res && !res.error && res.network) {
        return normalizeXbullNetworkName(res.network, res.networkPassphrase);
      }
    } catch {
      // Fall through to app network fallback.
    }
  }
  return process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet';
}

function getInjectedAlbedo(): AlbedoClient | null {
  if (typeof window === 'undefined') return null;
  return window.albedo ?? null;
}

async function getAlbedoClient(): Promise<AlbedoClient> {
  const injected = getInjectedAlbedo();
  if (injected) return injected;

  if (typeof document === 'undefined') {
    throw new Error('Albedo requires a browser environment');
  }

  if (!albedoScriptPromise) {
    albedoScriptPromise = new Promise((resolve, reject) => {
      const script = document.createElement('script');
      script.src = ALBEDO_INTENT_SCRIPT_URL;
      script.async = true;
      script.onload = () => {
        const client = getInjectedAlbedo();
        if (client) resolve(client);
        else reject(new Error('Albedo intent client failed to initialize'));
      };
      script.onerror = () =>
        reject(new Error('Failed to load Albedo intent client'));
      document.head.appendChild(script);
    });
  }

  return albedoScriptPromise;
}

function networkPassphraseToAlbedoNetwork(networkPassphrase?: string): string {
  return networkPassphrase?.includes('Public Global Stellar Network')
    ? 'public'
    : 'testnet';
}

export const WALLET_LABELS: Record<SupportedWallet, string> = {
  freighter: 'Freighter',
  xbull: 'xBull',
  albedo: 'Albedo',
  lobstr: 'LOBSTR',
};

export const WALLET_INSTALL_URLS: Record<SupportedWallet, string> = {
  freighter: 'https://www.freighter.app/',
  xbull: 'https://wallet.xbull.app/',
  albedo: 'https://albedo.link/',
  lobstr: 'https://lobstr.co/',
};

const FREIGHTER_DETECT_TIMEOUT_MS = 800;
const LOBSTR_DETECT_TIMEOUT_MS = 800;

function withTimeout<T>(promise: Promise<T>, ms: number, fallback: T): Promise<T> {
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => resolve(fallback), ms);
    promise
      .then((value) => {
        window.clearTimeout(timer);
        resolve(value);
      })
      .catch(() => {
        window.clearTimeout(timer);
        resolve(fallback);
      });
  });
}

function isFreighterInjected(): boolean {
  const win = getWindowRecord();
  return Boolean(win?.freighter);
}

async function detectFreighterInstalled(): Promise<boolean> {
  if (typeof window === 'undefined') return false;
  if (isFreighterInjected()) return true;

  try {
    // Freighter's isConnected() means "extension present" (not app authorization).
    const res = await withTimeout(
      freighterApi().isConnected(),
      FREIGHTER_DETECT_TIMEOUT_MS,
      { isConnected: false }
    );
    return !!res.isConnected && !res.error;
  } catch {
    return false;
  }
}

async function detectLobstrInstalled(timeoutMs = LOBSTR_DETECT_TIMEOUT_MS): Promise<boolean> {
  if (typeof window === 'undefined') return false;
  try {
    if (timeoutMs <= 0) {
      return await isLobstrConnected();
    }
    return await withTimeout(isLobstrConnected(), timeoutMs, false);
  } catch {
    return false;
  }
}

export async function getAvailableWallets(): Promise<AvailableWallet[]> {
  if (typeof window === 'undefined') {
    return [
      { id: 'freighter', label: WALLET_LABELS.freighter, installed: false },
      { id: 'xbull', label: WALLET_LABELS.xbull, installed: false },
      { id: 'albedo', label: WALLET_LABELS.albedo, installed: false },
      { id: 'lobstr', label: WALLET_LABELS.lobstr, installed: false },
    ];
  }

  const [freighterInstalled, lobstrInstalled] = await Promise.all([
    detectFreighterInstalled(),
    detectLobstrInstalled(),
  ]);

  return [
    {
      id: 'freighter',
      label: WALLET_LABELS.freighter,
      installed: freighterInstalled,
    },
    {
      id: 'xbull',
      label: WALLET_LABELS.xbull,
      installed: !!getXBullClient(),
    },
    {
      id: 'albedo',
      label: WALLET_LABELS.albedo,
      // Hosted intent wallet — always available in the browser.
      installed: true,
    },
    {
      id: 'lobstr',
      label: WALLET_LABELS.lobstr,
      installed: lobstrInstalled,
    },
  ];
}

export async function connectWallet(
  walletId: SupportedWallet
): Promise<WalletSession> {
  if (walletId === 'freighter') {
    const access = await freighterApi().requestAccess();

    if (access.error) {
      throw new Error(access.error.message ?? 'Freighter access denied');
    }

    const addressRes = await freighterApi().getAddress();
    if (addressRes.error) {
      throw new Error(addressRes.error.message ?? 'Failed to get address');
    }

    const networkRes = await freighterApi().getNetworkDetails();
    if (networkRes.error) {
      throw new Error(networkRes.error.message ?? 'Failed to get network');
    }

    return {
      walletId,
      address: addressRes.address,
      network: normalizeFreighterNetwork(networkRes.network),
      isConnected: true,
    };
  }

  if (walletId === 'xbull') {
    const xbull = getXBullClient();

    if (!xbull) {
      throw new Error('xBull not installed');
    }

    const result = await xbull.connect();
    const network = await resolveXbullNetwork(xbull);
    return {
      walletId,
      address: result.publicKey,
      network,
      isConnected: true,
    };
  }

  if (walletId === 'albedo') {
    const albedo = await getAlbedoClient();
    const result = await albedo.publicKey();
    const address = result.pubkey ?? result.publicKey;

    if (!address) {
      throw new Error('Albedo did not return a public key');
    }

    return {
      walletId,
      address,
      network: process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet',
      isConnected: true,
    };
  }

  if (walletId === 'lobstr') {
    const installed = await detectLobstrInstalled(0);
    if (!installed) {
      throw new Error('LOBSTR extension is not installed');
    }

    const address = await getLobstrPublicKey();
    if (!address) {
      throw new Error(
        'LOBSTR did not return a public key. Open the LOBSTR extension and unlock your account.'
      );
    }

    return {
      walletId,
      address,
      network: process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet',
      isConnected: true,
    };
  }

  throw new Error(`Unsupported wallet: ${walletId}`);
}

function getCapabilityResolution(capability: Capability): string {
  switch (capability) {
    case 'sign_transaction':
      return 'Allow transaction signing in your wallet settings';
    case 'view_address':
      return 'Allow account access in your wallet settings';
    case 'view_network':
      return 'Switch to the matching network in your wallet';
    case 'request_access':
      return 'Reconnect your wallet to grant access';
  }
}

export async function checkWalletCapabilities(
  walletId: SupportedWallet,
  network: string
): Promise<Capabilities> {
  const statuses: CapabilityStatus[] = [];

  if (walletId === 'freighter') {
    try {
      const accessRes = await freighterApi().requestAccess();
      statuses.push({
        capability: 'request_access',
        allowed: !accessRes.error,
        reason: accessRes.error?.message,
        resolution: accessRes.error
          ? getCapabilityResolution('request_access')
          : undefined,
      });
    } catch (e) {
      statuses.push({
        capability: 'request_access',
        allowed: false,
        reason: 'Failed to check wallet access',
        resolution: getCapabilityResolution('request_access'),
      });
    }

    try {
      const addressRes = await freighterApi().getAddress();
      const hasAddress = !addressRes.error && !!addressRes.address;
      statuses.push({
        capability: 'view_address',
        allowed: hasAddress,
        reason: addressRes.error?.message,
        resolution: hasAddress
          ? undefined
          : getCapabilityResolution('view_address'),
      });
    } catch (e) {
      statuses.push({
        capability: 'view_address',
        allowed: false,
        reason: 'Failed to get address',
        resolution: getCapabilityResolution('view_address'),
      });
    }

    try {
      const networkRes = await freighterApi().getNetworkDetails();
      const hasNetwork = !networkRes.error && !!networkRes.network;
      const walletNetwork = hasNetwork
        ? normalizeAppNetwork(networkRes.network)
        : null;
      const expectedNetwork = normalizeAppNetwork(network);
      const networkMatch =
        walletNetwork !== null &&
        expectedNetwork !== null &&
        walletNetwork === expectedNetwork;
      statuses.push({
        capability: 'view_network',
        allowed: networkMatch,
        reason: networkMatch
          ? undefined
          : `Wallet on ${networkRes.network}, expected ${network}`,
        resolution: networkMatch
          ? undefined
          : 'Switch wallet network to match the app',
      });
    } catch (e) {
      statuses.push({
        capability: 'view_network',
        allowed: false,
        reason: 'Failed to get network details',
        resolution: getCapabilityResolution('view_network'),
      });
    }

    const signCap = statuses.find((s) => s.capability === 'view_address');
    const netCap = statuses.find((s) => s.capability === 'view_network');
    statuses.push({
      capability: 'sign_transaction',
      allowed: Boolean(signCap?.allowed && netCap?.allowed),
      reason: !signCap?.allowed
        ? 'No address available'
        : !netCap?.allowed
          ? 'Network mismatch'
          : undefined,
      resolution:
        !signCap?.allowed || !netCap?.allowed
          ? getCapabilityResolution('sign_transaction')
          : undefined,
    });
  } else if (walletId === 'xbull') {
    const xbull = getXBullClient();
    const installed = !!xbull;
    statuses.push({
      capability: 'request_access',
      allowed: installed,
      reason: installed ? undefined : 'xBull not installed',
      resolution: installed ? undefined : 'Install the xBull extension',
    });
    statuses.push({
      capability: 'view_address',
      allowed: installed,
      reason: installed ? undefined : 'xBull not installed',
      resolution: installed
        ? undefined
        : getCapabilityResolution('view_address'),
    });

    let walletNetwork: string | null = null;
    if (xbull && typeof xbull.getNetwork === 'function') {
      try {
        const res = await xbull.getNetwork();
        if (res && !res.error && res.network) {
          walletNetwork = normalizeXbullNetworkName(
            res.network,
            res.networkPassphrase
          );
        }
      } catch {
        walletNetwork = null;
      }
    }

    const supportedNetwork =
      network === 'testnet' || network === 'mainnet' || network === 'public';
    const normalizedExpected =
      network === 'mainnet' || network === 'public' ? 'public' : network;
    const networkMatch =
      walletNetwork !== null
        ? normalizeXbullNetworkName(walletNetwork) === normalizedExpected
        : supportedNetwork;

    statuses.push({
      capability: 'view_network',
      allowed: networkMatch,
      reason: networkMatch
        ? undefined
        : walletNetwork
          ? `Wallet on ${walletNetwork}, expected ${network}`
          : `xBull supports testnet/public, expected ${network}`,
      resolution: networkMatch
        ? undefined
        : 'Switch wallet network to match the app',
    });
    statuses.push({
      capability: 'sign_transaction',
      allowed: Boolean(installed && networkMatch),
      reason: !installed
        ? 'xBull not installed'
        : !networkMatch
          ? 'Network mismatch'
          : undefined,
      resolution:
        !installed || !networkMatch
          ? getCapabilityResolution('sign_transaction')
          : undefined,
    });
  } else if (walletId === 'albedo') {
    const supportedNetwork =
      network === 'testnet' || network === 'mainnet' || network === 'public';
    statuses.push({ capability: 'request_access', allowed: true });
    statuses.push({ capability: 'view_address', allowed: true });
    statuses.push({
      capability: 'view_network',
      allowed: supportedNetwork,
      reason: supportedNetwork
        ? undefined
        : `Albedo supports testnet/public, expected ${network}`,
      resolution: supportedNetwork
        ? undefined
        : 'Switch app to testnet or mainnet',
    });
    statuses.push({
      capability: 'sign_transaction',
      allowed: supportedNetwork,
      reason: supportedNetwork ? undefined : 'Unsupported Albedo network',
      resolution: supportedNetwork
        ? undefined
        : 'Switch app to testnet or mainnet',
    });
  } else if (walletId === 'lobstr') {
    const installed = await detectLobstrInstalled(0);
    let hasAddress = false;
    if (installed) {
      try {
        const address = await getLobstrPublicKey();
        hasAddress = !!address;
      } catch {
        hasAddress = false;
      }
    }

    const supportedNetwork =
      network === 'testnet' || network === 'mainnet' || network === 'public';

    statuses.push({
      capability: 'request_access',
      allowed: installed,
      reason: installed ? undefined : 'LOBSTR extension is not installed',
      resolution: installed ? undefined : 'Install the LOBSTR browser extension',
    });
    statuses.push({
      capability: 'view_address',
      allowed: hasAddress,
      reason: hasAddress
        ? undefined
        : installed
          ? 'Unlock LOBSTR and select an account'
          : 'LOBSTR extension is not installed',
      resolution: hasAddress
        ? undefined
        : getCapabilityResolution('view_address'),
    });
    statuses.push({
      capability: 'view_network',
      allowed: supportedNetwork,
      reason: supportedNetwork
        ? undefined
        : `LOBSTR supports testnet/public, expected ${network}`,
      resolution: supportedNetwork
        ? undefined
        : 'Switch app to testnet or mainnet',
    });
    statuses.push({
      capability: 'sign_transaction',
      allowed: Boolean(installed && hasAddress && supportedNetwork),
      reason: !installed
        ? 'LOBSTR extension is not installed'
        : !hasAddress
          ? 'No address available'
          : !supportedNetwork
            ? 'Unsupported network'
            : undefined,
      resolution:
        !installed || !hasAddress || !supportedNetwork
          ? getCapabilityResolution('sign_transaction')
          : undefined,
    });
  }

  return {
    checkedAt: Date.now(),
    statuses,
  };
}

export function disconnectWallet(): WalletSession {
  return {
    walletId: null,
    address: null,
    network: null,
    isConnected: false,
  };
}

function normalizeWalletSignError(message: string): string {
  const lower = message.toLowerCase();
  if (
    lower.includes('user declined') ||
    lower.includes('declined access') ||
    lower.includes('signing denied') ||
    lower.includes('user rejected') ||
    lower.includes('transaction was rejected') ||
    lower.includes('cancel') ||
    lower.includes('reject') ||
    lower.includes('denied')
  ) {
    return 'User declined transaction signing';
  }
  return message;
}

export async function signTransactionWithWallet(
  xdr: string,
  walletId: SupportedWallet,
  networkPassphrase?: string,
  publicKey?: string
): Promise<string> {
  if (walletId === 'freighter') {
    const res = await freighterApi().signTransaction(xdr, { networkPassphrase });
    if (res.error) {
      throw new Error(
        normalizeWalletSignError(
          res.error.message ?? 'Transaction signing failed'
        )
      );
    }
    return res.signedTxXdr;
  }

  if (walletId === 'xbull') {
    const xbull = getXBullClient();

    if (!xbull?.sign) {
      throw new Error('xBull not installed');
    }

    // Determine network based on passphrase (heuristic used by wallets)
    const network = networkPassphrase?.includes('Test SDF Network')
      ? 'testnet'
      : 'public';

    try {
      const signedXdr = await xbull.sign({ xdr, network, publicKey });
      return signedXdr;
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : 'Transaction signing failed';
      throw new Error(normalizeWalletSignError(message));
    }
  }

  if (walletId === 'albedo') {
    const albedo = await getAlbedoClient();
    const network = networkPassphraseToAlbedoNetwork(networkPassphrase);

    try {
      const res = await albedo.tx({ xdr, network, pubkey: publicKey });
      const signedXdr = res.signed_envelope_xdr ?? res.signedXdr ?? res.xdr;
      if (!signedXdr) {
        throw new Error('Albedo did not return a signed XDR');
      }
      return signedXdr;
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : 'Transaction signing failed';
      throw new Error(normalizeWalletSignError(message));
    }
  }

  if (walletId === 'lobstr') {
    try {
      const signedXdr = await signLobstrTransaction(xdr);
      if (!signedXdr) {
        throw new Error('LOBSTR did not return a signed XDR');
      }
      return signedXdr;
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : 'Transaction signing failed';
      throw new Error(normalizeWalletSignError(message));
    }
  }

  throw new Error(`Transaction signing not supported for wallet: ${walletId}`);
}

/** Stub for callers that only need the XDR echoed back (e.g. tests / out-of-scope flows) */
export async function signTransactionStub(xdr: string) {
  return {
    ok: false,
    message: 'Signing stub only (out of scope)',
    xdr,
  };
}

/** Check if the current wallet address has changed */
export async function checkAddressChange(
  walletId: SupportedWallet,
  currentAddress: string | null
): Promise<string | null> {
  if (!currentAddress) return null;

  try {
    if (walletId === 'freighter') {
      const addressRes = await freighterApi().getAddress();
      if (addressRes.error) return null;
      return addressRes.address !== currentAddress ? addressRes.address : null;
    }

    if (walletId === 'xbull' || walletId === 'albedo' || walletId === 'lobstr') {
      // These wallets do not expose a reliable passive address check; reconnect instead.
      return null;
    }
  } catch {
    return null;
  }

  return null;
}

/** Refresh the current session to get updated account info */
export async function refreshWalletSession(
  walletId: SupportedWallet
): Promise<WalletSession> {
  if (walletId === 'freighter') {
    const addressRes = await freighterApi().getAddress();
    if (addressRes.error) {
      throw new Error(addressRes.error.message ?? 'Failed to get address');
    }

    const networkRes = await freighterApi().getNetworkDetails();
    if (networkRes.error) {
      throw new Error(networkRes.error.message ?? 'Failed to get network');
    }

    return {
      walletId,
      address: addressRes.address,
      network: normalizeFreighterNetwork(networkRes.network),
      isConnected: true,
    };
  }

  if (walletId === 'xbull') {
    const xbull = getXBullClient();

    if (!xbull) {
      throw new Error('xBull not installed');
    }

    const result = await xbull.connect();
    const network = await resolveXbullNetwork(xbull);
    return {
      walletId,
      address: result.publicKey,
      network,
      isConnected: true,
    };
  }

  if (walletId === 'albedo') {
    const albedo = await getAlbedoClient();
    const result = await albedo.publicKey();
    const address = result.pubkey ?? result.publicKey;

    if (!address) {
      throw new Error('Albedo did not return a public key');
    }

    return {
      walletId,
      address,
      network: process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet',
      isConnected: true,
    };
  }

  if (walletId === 'lobstr') {
    const installed = await detectLobstrInstalled(0);
    if (!installed) {
      throw new Error('LOBSTR extension is not installed');
    }

    const address = await getLobstrPublicKey();
    if (!address) {
      throw new Error(
        'LOBSTR did not return a public key. Open the LOBSTR extension and unlock your account.'
      );
    }

    return {
      walletId,
      address,
      network: process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet',
      isConnected: true,
    };
  }

  throw new Error(`Unsupported wallet: ${walletId}`);
}

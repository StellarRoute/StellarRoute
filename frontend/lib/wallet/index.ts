import * as freighterModule from '@stellar/freighter-api';
import {
  isConnected as isLobstrConnected,
  getPublicKey as getLobstrPublicKey,
  signTransaction as signLobstrTransaction,
} from '@lobstrco/signer-extension-api';

import type {
  AvailableWallet,
  SupportedWallet,
  WalletCapabilities,
  WalletCapabilityStatus,
  WalletNetwork,
  WalletSession,
} from "./types";

export const WALLET_LABELS: Record<SupportedWallet, string> = {
  freighter: 'Freighter',
  xbull: 'xBull',
  albedo: 'Albedo',
  lobstr: 'LOBSTR',
};

export const WALLET_CAPABILITIES_MAP: Record<SupportedWallet, WalletCapabilities> = {
  freighter: {
    canSign: true,
    supportedNetworks: ["testnet", "mainnet", "futurenet"],
    supportsNetworkSwitching: true,
  },
  xbull: {
    canSign: false, // xBull signTransaction not yet supported in-app
    supportedNetworks: ["testnet", "mainnet"],
    supportsNetworkSwitching: false,
  },
};

export function checkWalletCapabilities(
  walletId: SupportedWallet | null,
  network: WalletNetwork
): WalletCapabilityStatus {
  if (!walletId) {
    return {
      canSign: false,
      networkSupported: false,
      missingCapabilities: ["No wallet connected"],
    };
  }

  const capabilities = WALLET_CAPABILITIES_MAP[walletId];
  if (!capabilities) {
    return {
      canSign: false,
      networkSupported: false,
      missingCapabilities: ["Unsupported wallet"],
    };
  }

  const missing: string[] = [];
  const networkSupported = capabilities.supportedNetworks.includes(network);

  if (!capabilities.canSign) {
    missing.push("Transaction signing is not supported for this wallet.");
  }
  if (!networkSupported) {
    missing.push(`Network "${network}" is not supported by ${WALLET_LABELS[walletId] || walletId}.`);
  }

  return {
    canSign: capabilities.canSign,
    networkSupported,
    missingCapabilities: missing,
  };
}

export async function getAvailableWallets(): Promise<AvailableWallet[]> {
  const wallets: AvailableWallet[] = [];

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

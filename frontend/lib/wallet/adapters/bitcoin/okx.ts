import { getWindowRecord, hasCallable, readPath, withTimeout } from '../detect';
import {
  WalletAdapterError,
  normalizeProviderError,
} from '../errors';
import { resolveExecutionSupport } from '../execution-support';
import { createLiveSigningTracker } from '../live-state';
import type {
  AdapterCapabilities,
  AdapterCapabilityStatus,
  AdapterNetworkId,
  ChainNetworkInfo,
  ChainWalletAdapter,
  ChainWalletSession,
  SignMessageRequest,
  SignTransactionRequest,
  SignedMessageResult,
  SignedTransactionResult,
} from '../types';
import {
  bitcoinNetworkToUnisat,
  networksMatch,
  normalizeBitcoinNetwork,
} from './networks';
import type { OkxBitcoinProvider } from './types';

const ADAPTER_ID = 'okx-bitcoin';
const DETECT_TIMEOUT_MS = 800;

function getOkxBitcoin(): OkxBitcoinProvider | undefined {
  const win = getWindowRecord();
  const provider = readPath(win, ['okxwallet', 'bitcoin']) as
    | OkxBitcoinProvider
    | undefined;
  if (!provider) return undefined;
  if (!hasCallable(provider, 'connect') && !hasCallable(provider, 'signPsbt')) {
    return undefined;
  }
  return provider;
}

async function resolveAddress(
  provider: OkxBitcoinProvider
): Promise<{ address: string; publicKey?: string } | null> {
  if (hasCallable(provider, 'getAccounts')) {
    try {
      const accounts = await provider.getAccounts!();
      if (accounts?.[0]) {
        let publicKey: string | undefined;
        if (hasCallable(provider, 'getPublicKey')) {
          try {
            publicKey = await provider.getPublicKey!();
          } catch {
            publicKey = undefined;
          }
        }
        return { address: accounts[0], publicKey };
      }
    } catch {
      // fall through to connect
    }
  }
  return null;
}

async function readNetwork(
  provider: OkxBitcoinProvider
): Promise<AdapterNetworkId> {
  if (hasCallable(provider, 'getNetwork')) {
    try {
      const raw = await provider.getNetwork!();
      return normalizeBitcoinNetwork(raw);
    } catch {
      // default
    }
  }
  return 'bitcoin:mainnet';
}

async function readSession(
  provider: OkxBitcoinProvider
): Promise<ChainWalletSession | null> {
  const account = await resolveAddress(provider);
  if (!account) return null;
  const network = await readNetwork(provider);
  return {
    adapterId: ADAPTER_ID,
    chainFamily: 'bitcoin',
    account,
    network,
    isConnected: true,
  };
}

function buildCapabilities(
  installed: boolean,
  session: ChainWalletSession | null,
  expectedNetwork?: AdapterNetworkId
): AdapterCapabilities {
  const statuses: AdapterCapabilityStatus[] = [];
  const connected = Boolean(session?.isConnected && session.account.address);
  const networkOk =
    connected && networksMatch(session!.network, expectedNetwork);

  statuses.push({
    capability: 'connect',
    allowed: installed,
    reason: installed ? undefined : 'OKX Wallet (Bitcoin) not detected',
    resolution: installed ? undefined : 'Install OKX Wallet and enable Bitcoin',
  });
  statuses.push({ capability: 'disconnect', allowed: true });
  statuses.push({
    capability: 'view_address',
    allowed: connected,
    reason: connected ? undefined : 'No Bitcoin account connected',
  });
  statuses.push({
    capability: 'view_network',
    allowed: Boolean(connected && networkOk),
    reason:
      connected && !networkOk
        ? `Wallet on ${session!.network}, expected ${expectedNetwork}`
        : undefined,
    resolution:
      connected && !networkOk
        ? 'Switch OKX Bitcoin network to match the app'
        : undefined,
  });
  statuses.push({
    capability: 'sign_message',
    allowed: Boolean(connected && networkOk),
  });
  statuses.push({
    capability: 'sign_transaction',
    allowed: Boolean(connected && networkOk),
    resolution:
      !connected || !networkOk
        ? 'Connect and match networks before signing a PSBT'
        : undefined,
  });
  statuses.push({
    capability: 'switch_network',
    allowed: installed && hasCallable(getOkxBitcoin(), 'switchNetwork'),
  });

  return { checkedAt: Date.now(), statuses };
}

export function createOkxBitcoinAdapter(): ChainWalletAdapter {
  const live = createLiveSigningTracker();
  let lastExpected: AdapterNetworkId | undefined;

  const refreshLive = (
    session: ChainWalletSession | null,
    expectedNetwork?: AdapterNetworkId
  ) => {
    const expected = expectedNetwork ?? lastExpected;
    if (expected) lastExpected = expected;
    const connected = Boolean(session?.isConnected && session.account.address);
    const networkMatch = connected
      ? networksMatch(session!.network, expected)
      : true;
    live.patch({
      connected,
      networkMatch,
      canSign: connected && networkMatch,
    });
  };

  return {
    id: ADAPTER_ID,
    label: 'OKX Wallet (Bitcoin)',
    chainFamily: 'bitcoin',
    installUrl: 'https://www.okx.com/download',

    async detectInstalled() {
      if (typeof window === 'undefined') return false;
      const provider = getOkxBitcoin();
      if (!provider) return false;
      if (hasCallable(provider, 'getAccounts')) {
        await withTimeout(provider.getAccounts!(), DETECT_TIMEOUT_MS, []);
      }
      return true;
    },

    async connect(expectedNetwork?: AdapterNetworkId) {
      const provider = getOkxBitcoin();
      if (!provider) {
        throw new WalletAdapterError(
          'OKX Wallet Bitcoin provider is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }

      try {
        const connected = await provider.connect();
        let address: string | undefined;
        let publicKey: string | undefined;

        if (Array.isArray(connected)) {
          address = connected[0];
        } else if (connected && typeof connected === 'object') {
          address = connected.address;
          publicKey = connected.publicKey;
        }

        if (!address) {
          const sessionProbe = await resolveAddress(provider);
          address = sessionProbe?.address;
          publicKey = publicKey ?? sessionProbe?.publicKey;
        }

        if (!address) {
          throw new WalletAdapterError(
            'OKX Wallet did not return a Bitcoin address',
            'provider_error',
            ADAPTER_ID
          );
        }

        if (expectedNetwork && hasCallable(provider, 'switchNetwork')) {
          const target = bitcoinNetworkToUnisat(expectedNetwork);
          if (target) {
            try {
              const current = await readNetwork(provider);
              if (current !== expectedNetwork) {
                await provider.switchNetwork!(target);
              }
            } catch (err) {
              const normalized = normalizeProviderError(
                err,
                'Network switch failed',
                ADAPTER_ID
              );
              if (normalized.code === 'user_rejected') throw normalized;
            }
          }
        }

        const network = await readNetwork(provider);

        if (!publicKey && hasCallable(provider, 'getPublicKey')) {
          try {
            publicKey = await provider.getPublicKey!();
          } catch {
            publicKey = undefined;
          }
        }

        const session: ChainWalletSession = {
          adapterId: ADAPTER_ID,
          chainFamily: 'bitcoin',
          account: { address, publicKey },
          network,
          isConnected: true,
        };
        if (expectedNetwork) lastExpected = expectedNetwork;
        refreshLive(session, expectedNetwork);
        return session;
      } catch (err) {
        live.patch({ connected: false, canSign: false });
        throw normalizeProviderError(
          err,
          'Failed to connect OKX Bitcoin wallet',
          ADAPTER_ID
        );
      }
    },

    async disconnect() {
      live.reset();
      return;
    },

    async getSession() {
      const provider = getOkxBitcoin();
      if (!provider) {
        live.patch({ connected: false, canSign: false });
        return null;
      }
      try {
        const session = await readSession(provider);
        refreshLive(session, lastExpected);
        return session;
      } catch {
        live.patch({ connected: false, canSign: false });
        return null;
      }
    },

    async getNetwork(expectedNetwork?: AdapterNetworkId) {
      const provider = getOkxBitcoin();
      if (!provider) {
        throw new WalletAdapterError(
          'OKX Wallet Bitcoin provider is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }
      if (expectedNetwork) lastExpected = expectedNetwork;
      const network = await readNetwork(provider);
      const expected = expectedNetwork ?? lastExpected;
      const info = {
        network,
        matchesExpected: networksMatch(network, expected),
        expected,
      } satisfies ChainNetworkInfo;
      const session = await readSession(provider).catch(() => null);
      refreshLive(session, expected);
      return info;
    },

    async switchNetwork(network: AdapterNetworkId) {
      const provider = getOkxBitcoin();
      if (!provider || !hasCallable(provider, 'switchNetwork')) {
        throw new WalletAdapterError(
          'OKX Bitcoin network switching is unavailable',
          'unsupported_capability',
          ADAPTER_ID
        );
      }
      const target = bitcoinNetworkToUnisat(network);
      if (!target) {
        throw new WalletAdapterError(
          `Unsupported Bitcoin network: ${network}`,
          'invalid_request',
          ADAPTER_ID
        );
      }
      try {
        await provider.switchNetwork!(target);
        return this.getNetwork(network);
      } catch (err) {
        throw normalizeProviderError(
          err,
          'Failed to switch OKX Bitcoin network',
          ADAPTER_ID
        );
      }
    },

    async signMessage(request: SignMessageRequest): Promise<SignedMessageResult> {
      const provider = getOkxBitcoin();
      if (!provider) {
        throw new WalletAdapterError(
          'OKX Wallet Bitcoin provider is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }
      const session = await readSession(provider);
      if (!session) {
        throw new WalletAdapterError(
          'Connect OKX Wallet before signing',
          'not_connected',
          ADAPTER_ID
        );
      }
      if (lastExpected && !networksMatch(session.network, lastExpected)) {
        throw new WalletAdapterError(
          'Wallet network does not match the app. Switch networks before signing.',
          'network_mismatch',
          ADAPTER_ID
        );
      }
      try {
        const signature = await provider.signMessage(
          request.message,
          request.bitcoinSignType ?? 'ecdsa'
        );
        return {
          signature,
          address: session.account.address,
          publicKey: session.account.publicKey,
        };
      } catch (err) {
        throw normalizeProviderError(
          err,
          'OKX Bitcoin message signing failed',
          ADAPTER_ID
        );
      }
    },

    async signTransaction(
      request: SignTransactionRequest
    ): Promise<SignedTransactionResult> {
      if (request.kind !== 'bitcoin_psbt') {
        throw new WalletAdapterError(
          'OKX Bitcoin adapter only signs Bitcoin PSBTs',
          'invalid_request',
          ADAPTER_ID
        );
      }
      if (request.format === 'base64') {
        throw new WalletAdapterError(
          'OKX Bitcoin adapter requires PSBT hex (format: "hex")',
          'invalid_request',
          ADAPTER_ID
        );
      }
      const provider = getOkxBitcoin();
      if (!provider) {
        throw new WalletAdapterError(
          'OKX Wallet Bitcoin provider is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }
      const session = await readSession(provider);
      if (!session) {
        throw new WalletAdapterError(
          'Connect OKX Wallet before signing',
          'not_connected',
          ADAPTER_ID
        );
      }
      if (lastExpected && !networksMatch(session.network, lastExpected)) {
        throw new WalletAdapterError(
          'Wallet network does not match the app. Switch networks before signing.',
          'network_mismatch',
          ADAPTER_ID
        );
      }
      try {
        const signed = await provider.signPsbt(
          request.psbt,
          request.options as Record<string, unknown> | undefined
        );
        return { kind: 'bitcoin_psbt', psbt: signed, format: 'hex' };
      } catch (err) {
        throw normalizeProviderError(
          err,
          'OKX Bitcoin PSBT signing failed',
          ADAPTER_ID
        );
      }
    },

    async checkCapabilities(expectedNetwork?: AdapterNetworkId) {
      const installed = await this.detectInstalled();
      let session: ChainWalletSession | null = null;
      if (installed) {
        try {
          session = await this.getSession();
        } catch {
          session = null;
        }
      }
      const expected = expectedNetwork ?? lastExpected;
      if (expected) lastExpected = expected;
      refreshLive(session, expected);
      return buildCapabilities(installed, session, expected);
    },

    getExecutionSupport(routeHint) {
      return resolveExecutionSupport('bitcoin', routeHint, live.read());
    },
  };
}

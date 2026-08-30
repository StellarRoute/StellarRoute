import { getWindowRecord, hasCallable, withTimeout } from '../detect';
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
import type { UnisatProvider } from './types';

const ADAPTER_ID = 'unisat';
const DETECT_TIMEOUT_MS = 800;

function getUnisat(): UnisatProvider | undefined {
  const win = getWindowRecord();
  const provider = win?.unisat as UnisatProvider | undefined;
  if (!provider) return undefined;
  if (!hasCallable(provider, 'requestAccounts')) return undefined;
  return provider;
}

async function readSession(
  provider: UnisatProvider
): Promise<ChainWalletSession | null> {
  const accounts = await provider.getAccounts();
  const address = accounts?.[0];
  if (!address) return null;

  const rawNetwork = await provider.getNetwork();
  const network = normalizeBitcoinNetwork(rawNetwork);
  let publicKey: string | undefined;
  if (hasCallable(provider, 'getPublicKey')) {
    try {
      publicKey = await provider.getPublicKey!();
    } catch {
      publicKey = undefined;
    }
  }

  return {
    adapterId: ADAPTER_ID,
    chainFamily: 'bitcoin',
    account: { address, publicKey },
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
    reason: installed ? undefined : 'UniSat extension not detected',
    resolution: installed ? undefined : 'Install UniSat from unisat.io',
  });
  statuses.push({
    capability: 'disconnect',
    allowed: true,
  });
  statuses.push({
    capability: 'view_address',
    allowed: connected,
    reason: connected ? undefined : 'No Bitcoin account connected',
    resolution: connected ? undefined : 'Connect UniSat and unlock your wallet',
  });
  statuses.push({
    capability: 'view_network',
    allowed: Boolean(connected && networkOk),
    reason:
      connected && !networkOk
        ? `Wallet on ${session!.network}, expected ${expectedNetwork}`
        : connected
          ? undefined
          : 'Not connected',
    resolution:
      connected && !networkOk
        ? 'Switch UniSat to the matching Bitcoin network'
        : undefined,
  });
  statuses.push({
    capability: 'sign_message',
    allowed: Boolean(connected && networkOk),
    reason: !connected
      ? 'Not connected'
      : !networkOk
        ? 'Network mismatch'
        : undefined,
  });
  statuses.push({
    capability: 'sign_transaction',
    allowed: Boolean(connected && networkOk),
    reason: !connected
      ? 'Not connected'
      : !networkOk
        ? 'Network mismatch'
        : undefined,
    resolution:
      !connected || !networkOk
        ? 'Connect and match networks before signing a PSBT'
        : undefined,
  });
  statuses.push({
    capability: 'switch_network',
    allowed: installed && hasCallable(getUnisat(), 'switchNetwork'),
  });

  return { checkedAt: Date.now(), statuses };
}

export function createUnisatAdapter(): ChainWalletAdapter {
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
    label: 'UniSat',
    chainFamily: 'bitcoin',
    installUrl: 'https://unisat.io/download',

    async detectInstalled() {
      if (typeof window === 'undefined') return false;
      const provider = getUnisat();
      if (!provider) return false;
      try {
        // Touch getAccounts with a timeout — presence alone is enough.
        await withTimeout(provider.getAccounts(), DETECT_TIMEOUT_MS, []);
        return true;
      } catch {
        return true; // Provider exists even if call fails (locked).
      }
    },

    async connect(expectedNetwork?: AdapterNetworkId) {
      const provider = getUnisat();
      if (!provider) {
        throw new WalletAdapterError(
          'UniSat is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }

      try {
        const accounts = await provider.requestAccounts();
        const address = accounts?.[0];
        if (!address) {
          throw new WalletAdapterError(
            'UniSat did not return an account',
            'provider_error',
            ADAPTER_ID
          );
        }

        if (expectedNetwork && hasCallable(provider, 'switchNetwork')) {
          const target = bitcoinNetworkToUnisat(expectedNetwork);
          if (target) {
            try {
              const current = normalizeBitcoinNetwork(
                await provider.getNetwork()
              );
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

        const session = await readSession(provider);
        if (!session) {
          throw new WalletAdapterError(
            'UniSat connection failed',
            'provider_error',
            ADAPTER_ID
          );
        }

        if (expectedNetwork) lastExpected = expectedNetwork;
        refreshLive(session, expectedNetwork);
        return session;
      } catch (err) {
        live.patch({ connected: false, canSign: false });
        throw normalizeProviderError(
          err,
          'Failed to connect UniSat',
          ADAPTER_ID
        );
      }
    },

    async disconnect() {
      live.reset();
      // UniSat has no dapp-level disconnect API; session is app-local.
      return;
    },

    async getSession() {
      const provider = getUnisat();
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
      const provider = getUnisat();
      if (!provider) {
        throw new WalletAdapterError(
          'UniSat is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }
      if (expectedNetwork) lastExpected = expectedNetwork;
      const raw = await provider.getNetwork();
      const network = normalizeBitcoinNetwork(raw);
      const expected = expectedNetwork ?? lastExpected;
      const info = {
        network,
        raw: String(raw),
        matchesExpected: networksMatch(network, expected),
        expected,
      } satisfies ChainNetworkInfo;
      const session = await readSession(provider).catch(() => null);
      refreshLive(session, expected);
      return info;
    },

    async switchNetwork(network: AdapterNetworkId) {
      const provider = getUnisat();
      if (!provider || !hasCallable(provider, 'switchNetwork')) {
        throw new WalletAdapterError(
          'UniSat network switching is unavailable',
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
          'Failed to switch UniSat network',
          ADAPTER_ID
        );
      }
    },

    async signMessage(request: SignMessageRequest): Promise<SignedMessageResult> {
      const provider = getUnisat();
      if (!provider) {
        throw new WalletAdapterError(
          'UniSat is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }
      const session = await readSession(provider);
      if (!session) {
        throw new WalletAdapterError(
          'Connect UniSat before signing',
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
          'UniSat message signing failed',
          ADAPTER_ID
        );
      }
    },

    async signTransaction(
      request: SignTransactionRequest
    ): Promise<SignedTransactionResult> {
      if (request.kind !== 'bitcoin_psbt') {
        throw new WalletAdapterError(
          'UniSat only signs Bitcoin PSBTs',
          'invalid_request',
          ADAPTER_ID
        );
      }
      const provider = getUnisat();
      if (!provider) {
        throw new WalletAdapterError(
          'UniSat is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }
      const session = await readSession(provider);
      if (!session) {
        throw new WalletAdapterError(
          'Connect UniSat before signing',
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
      if (request.format === 'base64') {
        throw new WalletAdapterError(
          'UniSat adapter requires PSBT hex (format: "hex")',
          'invalid_request',
          ADAPTER_ID
        );
      }
      try {
        const signed = await provider.signPsbt(request.psbt, request.options);
        return { kind: 'bitcoin_psbt', psbt: signed, format: 'hex' };
      } catch (err) {
        throw normalizeProviderError(
          err,
          'UniSat PSBT signing failed',
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

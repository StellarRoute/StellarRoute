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
import { networksMatch, normalizeTronNetwork } from './networks';
import type { TronLinkProvider, TronWebLike } from './types';

const ADAPTER_ID = 'tronlink';
const DETECT_TIMEOUT_MS = 800;

function getTronLink(): TronLinkProvider | undefined {
  const win = getWindowRecord();
  if (!win) return undefined;
  const tronLink = win.tronLink as TronLinkProvider | undefined;
  if (tronLink) return tronLink;
  // Some builds only inject tronWeb
  if (win.tronWeb) {
    return { tronWeb: win.tronWeb as TronWebLike, ready: true };
  }
  return undefined;
}

function getTronWeb(provider?: TronLinkProvider): TronWebLike | undefined {
  const win = getWindowRecord();
  return (
    provider?.tronWeb ??
    (win?.tronWeb as TronWebLike | undefined) ??
    undefined
  );
}

function readAddress(tronWeb: TronWebLike | undefined): string | null {
  const address = tronWeb?.defaultAddress?.base58;
  return address && address.length > 0 ? address : null;
}

function readNetwork(tronWeb: TronWebLike | undefined): AdapterNetworkId {
  const host =
    tronWeb?.fullNode?.host ??
    tronWeb?.solidityNode?.host ??
    tronWeb?.eventServer?.host;
  return normalizeTronNetwork(host);
}

async function requestAccounts(
  provider: TronLinkProvider
): Promise<void> {
  if (!hasCallable(provider, 'request')) {
    return;
  }
  const result = (await provider.request!({
    method: 'tron_requestAccounts',
  })) as { code?: number; message?: string } | undefined;

  // TronLink: code 200 success; 4001 user reject (varies by version)
  if (result && typeof result === 'object' && result.code != null) {
    if (result.code === 4001) {
      throw new WalletAdapterError(
        'User rejected the wallet request',
        'user_rejected',
        ADAPTER_ID
      );
    }
    if (result.code !== 200 && result.code !== 0) {
      const msg = result.message ?? `TronLink request failed (${result.code})`;
      if (msg.toLowerCase().includes('reject')) {
        throw new WalletAdapterError(msg, 'user_rejected', ADAPTER_ID);
      }
      throw new WalletAdapterError(msg, 'provider_error', ADAPTER_ID);
    }
  }
}

async function readSession(
  provider: TronLinkProvider
): Promise<ChainWalletSession | null> {
  const tronWeb = getTronWeb(provider);
  const address = readAddress(tronWeb);
  if (!address) return null;
  return {
    adapterId: ADAPTER_ID,
    chainFamily: 'tron',
    account: {
      address,
      publicKey: tronWeb?.defaultAddress?.hex,
    },
    network: readNetwork(tronWeb),
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
  const tronWeb = getTronWeb(getTronLink());
  const canSignTx = Boolean(tronWeb?.trx && hasCallable(tronWeb.trx, 'sign'));
  const canSignMsg = Boolean(
    tronWeb?.trx &&
      (hasCallable(tronWeb.trx, 'signMessageV2') ||
        hasCallable(tronWeb.trx, 'signMessage'))
  );

  statuses.push({
    capability: 'connect',
    allowed: installed,
    reason: installed ? undefined : 'TronLink extension not detected',
    resolution: installed ? undefined : 'Install TronLink from tronlink.org',
  });
  statuses.push({ capability: 'disconnect', allowed: true });
  statuses.push({
    capability: 'view_address',
    allowed: connected,
    reason: connected ? undefined : 'Unlock TronLink and select an account',
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
        ? 'Switch TronLink to Nile/Shasta/Mainnet to match the app'
        : undefined,
  });
  statuses.push({
    capability: 'sign_message',
    allowed: Boolean(connected && networkOk && canSignMsg),
    reason: !canSignMsg ? 'TronLink message signing unavailable' : undefined,
  });
  statuses.push({
    capability: 'sign_transaction',
    allowed: Boolean(connected && networkOk && canSignTx),
    reason: !canSignTx
      ? 'TronLink transaction signing unavailable'
      : !connected
        ? 'Not connected'
        : !networkOk
          ? 'Network mismatch'
          : undefined,
    resolution:
      !connected || !networkOk || !canSignTx
        ? 'Connect TronLink on the matching network before signing'
        : undefined,
  });
  statuses.push({
    capability: 'switch_network',
    allowed: false,
    reason: 'TronLink requires manual network switching in the extension',
    resolution: 'Open TronLink settings and select the target network',
  });

  return { checkedAt: Date.now(), statuses };
}

export function createTronLinkAdapter(): ChainWalletAdapter {
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
    const tronWeb = getTronWeb(getTronLink());
    const canSignTx = Boolean(tronWeb?.trx && hasCallable(tronWeb.trx, 'sign'));
    live.patch({
      connected,
      networkMatch,
      canSign: connected && networkMatch && canSignTx,
    });
  };

  return {
    id: ADAPTER_ID,
    label: 'TronLink',
    chainFamily: 'tron',
    installUrl: 'https://www.tronlink.org/',

    async detectInstalled() {
      if (typeof window === 'undefined') return false;
      const provider = getTronLink();
      if (!provider) return false;
      // TronLink may inject before tronWeb is ready — still count as installed.
      if (provider.ready || getTronWeb(provider)) return true;
      await withTimeout(Promise.resolve(true), DETECT_TIMEOUT_MS, true);
      return Boolean(getTronLink());
    },

    async connect(expectedNetwork?: AdapterNetworkId) {
      const provider = getTronLink();
      if (!provider) {
        throw new WalletAdapterError(
          'TronLink is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }

      try {
        await requestAccounts(provider);

        // Wait briefly for tronWeb.defaultAddress after authorization.
        let session = await readSession(provider);
        if (!session) {
          // Brief post-auth settle; SSR-safe via withTimeout/globalThis timers.
          await withTimeout(new Promise<void>(() => {}), 300, undefined);
          session = await readSession(provider);
        }

        if (!session) {
          throw new WalletAdapterError(
            'TronLink did not return an account. Unlock the extension and try again.',
            'not_connected',
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
          'Failed to connect TronLink',
          ADAPTER_ID
        );
      }
    },

    async disconnect() {
      live.reset();
      return;
    },

    async getSession() {
      const provider = getTronLink();
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
      const provider = getTronLink();
      const tronWeb = getTronWeb(provider);
      if (!provider || !tronWeb) {
        throw new WalletAdapterError(
          'TronLink is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }
      if (expectedNetwork) lastExpected = expectedNetwork;
      const network = readNetwork(tronWeb);
      const expected = expectedNetwork ?? lastExpected;
      const info = {
        network,
        raw: tronWeb.fullNode?.host,
        matchesExpected: networksMatch(network, expected),
        expected,
      } satisfies ChainNetworkInfo;
      const session = await readSession(provider).catch(() => null);
      refreshLive(session, expected);
      return info;
    },

    async signMessage(request: SignMessageRequest): Promise<SignedMessageResult> {
      const provider = getTronLink();
      const tronWeb = getTronWeb(provider);
      if (!provider || !tronWeb?.trx) {
        throw new WalletAdapterError(
          'TronLink is not installed',
          'not_installed',
          ADAPTER_ID
        );
      }
      const session = await readSession(provider);
      if (!session) {
        throw new WalletAdapterError(
          'Connect TronLink before signing',
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
        let signature: string;
        if (hasCallable(tronWeb.trx, 'signMessageV2')) {
          signature = await tronWeb.trx.signMessageV2!(request.message);
        } else if (hasCallable(tronWeb.trx, 'signMessage')) {
          signature = await tronWeb.trx.signMessage!(request.message);
        } else {
          throw new WalletAdapterError(
            'TronLink message signing is unavailable',
            'unsupported_capability',
            ADAPTER_ID
          );
        }
        return {
          signature,
          address: session.account.address,
          publicKey: session.account.publicKey,
        };
      } catch (err) {
        throw normalizeProviderError(
          err,
          'TronLink message signing failed',
          ADAPTER_ID
        );
      }
    },

    async signTransaction(
      request: SignTransactionRequest
    ): Promise<SignedTransactionResult> {
      if (request.kind !== 'tron_transaction') {
        throw new WalletAdapterError(
          'TronLink adapter only signs TRON transactions',
          'invalid_request',
          ADAPTER_ID
        );
      }
      const provider = getTronLink();
      const tronWeb = getTronWeb(provider);
      if (!provider || !tronWeb?.trx || !hasCallable(tronWeb.trx, 'sign')) {
        throw new WalletAdapterError(
          'TronLink transaction signing is unavailable',
          'unsupported_capability',
          ADAPTER_ID
        );
      }
      const session = await readSession(provider);
      if (!session) {
        throw new WalletAdapterError(
          'Connect TronLink before signing',
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
        const signed = await tronWeb.trx.sign!(request.transaction);
        return { kind: 'tron_transaction', transaction: signed };
      } catch (err) {
        throw normalizeProviderError(
          err,
          'TronLink transaction signing failed',
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
      return resolveExecutionSupport('tron', routeHint, live.read());
    },
  };
}

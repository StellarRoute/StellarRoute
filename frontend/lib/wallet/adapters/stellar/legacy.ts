/**
 * Thin ChainWalletAdapter wrappers over the existing Stellar wallet module.
 * Preserves Freighter/xBull/Albedo/LOBSTR behavior without duplicating logic.
 */

import {
  checkWalletCapabilities,
  connectWallet,
  disconnectWallet,
  refreshWalletSession,
  signTransactionWithWallet,
} from '../../index';
import type { SupportedWallet } from '../../types';
import { WalletAdapterError } from '../errors';
import { resolveExecutionSupport } from '../execution-support';
import { createLiveSigningTracker } from '../live-state';
import type {
  AdapterNetworkId,
  ChainNetworkInfo,
  ChainWalletAdapter,
  ChainWalletSession,
  SignMessageRequest,
  SignTransactionRequest,
  SignedTransactionResult,
} from '../types';

const STELLAR_META: Record<
  SupportedWallet,
  { label: string; installUrl: string }
> = {
  freighter: {
    label: 'Freighter',
    installUrl: 'https://www.freighter.app/',
  },
  xbull: {
    label: 'xBull',
    installUrl: 'https://wallet.xbull.app/',
  },
  albedo: {
    label: 'Albedo',
    installUrl: 'https://albedo.link/',
  },
  lobstr: {
    label: 'LOBSTR',
    installUrl: 'https://lobstr.co/',
  },
};

function toAdapterNetwork(network: string | null | undefined): AdapterNetworkId {
  const key = (network ?? 'testnet').trim().toLowerCase();
  if (
    key === 'public' ||
    key === 'pubnet' ||
    key === 'mainnet' ||
    key === 'production'
  ) {
    return 'stellar:public';
  }
  if (key === 'futurenet') return 'stellar:futurenet';
  return 'stellar:testnet';
}

function fromExpectedNetwork(
  expected?: AdapterNetworkId
): string | undefined {
  if (!expected) return undefined;
  if (expected === 'stellar:public') return 'public';
  if (expected === 'stellar:futurenet') return 'futurenet';
  if (expected === 'stellar:testnet') return 'testnet';
  return undefined;
}

function sessionFromLegacy(
  walletId: SupportedWallet,
  session: {
    address: string | null;
    network: string | null;
    isConnected: boolean;
  }
): ChainWalletSession {
  if (!session.address || !session.isConnected) {
    throw new WalletAdapterError(
      `${walletId} did not return a connected session`,
      'not_connected',
      walletId
    );
  }
  return {
    adapterId: walletId,
    chainFamily: 'stellar',
    account: { address: session.address, publicKey: session.address },
    network: toAdapterNetwork(session.network),
    isConnected: true,
  };
}

export function createStellarWalletAdapter(
  walletId: SupportedWallet
): ChainWalletAdapter {
  const meta = STELLAR_META[walletId];
  const live = createLiveSigningTracker();
  let lastExpected: AdapterNetworkId | undefined;

  const refreshLive = (
    session: ChainWalletSession | null,
    expectedNetwork?: AdapterNetworkId
  ) => {
    const expected = expectedNetwork ?? lastExpected;
    if (expected) lastExpected = expected;
    const connected = Boolean(session?.isConnected && session.account.address);
    const networkMatch =
      connected && expected ? session!.network === expected : true;
    live.patch({
      connected,
      networkMatch,
      canSign: connected && networkMatch,
    });
  };

  return {
    id: walletId,
    label: meta.label,
    chainFamily: 'stellar',
    installUrl: meta.installUrl,

    async detectInstalled() {
      const { getAvailableWallets } = await import('../../index');
      const wallets = await getAvailableWallets();
      return Boolean(wallets.find((w) => w.id === walletId)?.installed);
    },

    async connect(expectedNetwork?: AdapterNetworkId) {
      void expectedNetwork; // Stellar network is read from the wallet / app env.
      if (expectedNetwork) lastExpected = expectedNetwork;
      const session = await connectWallet(walletId);
      const adapted = sessionFromLegacy(walletId, session);
      refreshLive(adapted, expectedNetwork ?? lastExpected);
      return adapted;
    },

    async disconnect() {
      live.reset();
      disconnectWallet();
    },

    async getSession() {
      try {
        const session = await refreshWalletSession(walletId);
        if (!session.isConnected || !session.address) {
          live.patch({ connected: false, canSign: false });
          return null;
        }
        const adapted = sessionFromLegacy(walletId, session);
        refreshLive(adapted, lastExpected);
        return adapted;
      } catch {
        live.patch({ connected: false, canSign: false });
        return null;
      }
    },

    async getNetwork(expectedNetwork?: AdapterNetworkId) {
      if (expectedNetwork) lastExpected = expectedNetwork;
      const session = await refreshWalletSession(walletId);
      const network = toAdapterNetwork(session.network);
      const expected = expectedNetwork ?? lastExpected;
      const expectedLegacy = fromExpectedNetwork(expected);
      const matchesExpected = expected
        ? expected === network ||
          (expectedLegacy != null &&
            toAdapterNetwork(session.network) === expected)
        : true;
      const info = {
        network,
        raw: session.network ?? undefined,
        expected,
        matchesExpected,
      } satisfies ChainNetworkInfo;
      if (session.isConnected && session.address) {
        refreshLive(sessionFromLegacy(walletId, session), expected);
      } else {
        live.patch({ connected: false, networkMatch: matchesExpected, canSign: false });
      }
      return info;
    },

    async signMessage(request: SignMessageRequest) {
      void request;
      throw new WalletAdapterError(
        'Stellar wallets in StellarRoute sign transactions (XDR), not arbitrary messages',
        'unsupported_capability',
        walletId
      );
    },

    async signTransaction(request: SignTransactionRequest) {
      if (request.kind !== 'stellar_xdr') {
        throw new WalletAdapterError(
          `Stellar adapter cannot handle payload kind "${request.kind}"`,
          'invalid_request',
          walletId
        );
      }
      if (lastExpected) {
        const info = await this.getNetwork(lastExpected);
        if (!info.matchesExpected) {
          throw new WalletAdapterError(
            'Wallet network does not match the app. Switch networks before signing.',
            'network_mismatch',
            walletId
          );
        }
      }
      const signedXdr = await signTransactionWithWallet(
        request.xdr,
        walletId,
        request.networkPassphrase,
        request.publicKey
      );
      return {
        kind: 'stellar_xdr',
        signedXdr,
      } satisfies SignedTransactionResult;
    },

    async checkCapabilities(expectedNetwork?: AdapterNetworkId) {
      const expected = expectedNetwork ?? lastExpected;
      if (expected) lastExpected = expected;
      const legacyNetwork =
        fromExpectedNetwork(expected) ??
        process.env.NEXT_PUBLIC_STELLAR_NETWORK ??
        'testnet';
      const caps = await checkWalletCapabilities(walletId, legacyNetwork);
      const session = await this.getSession();
      refreshLive(session, expected);
      return {
        checkedAt: caps.checkedAt,
        statuses: [
          {
            capability: 'connect' as const,
            allowed: true,
          },
          {
            capability: 'disconnect' as const,
            allowed: true,
          },
          ...caps.statuses.map((status) => ({
            capability:
              status.capability === 'request_access'
                ? ('connect' as const)
                : status.capability === 'sign_transaction'
                  ? ('sign_transaction' as const)
                  : status.capability === 'view_address'
                    ? ('view_address' as const)
                    : ('view_network' as const),
            allowed: status.allowed,
            reason: status.reason,
            resolution: status.resolution,
          })),
          {
            capability: 'send_transaction' as const,
            allowed: false,
            reason: 'Stellar broadcast is handled by Horizon submit helpers',
            resolution: 'Use lib/wallet/submit after signing XDR',
          },
          {
            capability: 'sign_message' as const,
            allowed: false,
            reason: 'Not used for Stellar swap signing',
          },
          {
            capability: 'switch_network' as const,
            allowed: false,
            reason: 'Switch network inside the Stellar wallet extension',
          },
        ],
      };
    },

    getExecutionSupport(routeHint) {
      return resolveExecutionSupport('stellar', routeHint, live.read());
    },
  };
}

export function createAllStellarAdapters(): ChainWalletAdapter[] {
  return (['freighter', 'xbull', 'albedo', 'lobstr'] as SupportedWallet[]).map(
    (id) => createStellarWalletAdapter(id)
  );
}

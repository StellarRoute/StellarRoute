/**
 * Execution support for multi-chain wallet sessions.
 *
 * Live paths: Stellar-native swaps, plus testnet Stellar→EVM CCTP when the
 * API reports bridge readiness. Other adapters may sign chain-native
 * payloads, but MUST surface `unsupported` / `degraded` when no backend
 * cross-chain route exists.
 */

import type { ChainFamily, ExecutionSupport } from './types';

const CROSS_CHAIN_BACKEND_ROUTES: ReadonlySet<string> = new Set([
  // Proven testnet CCTP directions (Stellar ↔ Sepolia USDC).
  // Settlement still requires API readiness (`CCTP_ENABLED` + /api/v2).
  // Format: `${source}->${destination}`
  'stellar->evm',
  'evm->stellar',
]);

let cctpExecutableRoutes: ReadonlySet<string> = new Set();

/** Updated from `GET /api/v2` corridor executability — test-only reset via clear. */
export function setCctpExecutableRoutes(
  routes: Array<{ source: ChainFamily; destination: ChainFamily }>
): void {
  cctpExecutableRoutes = new Set(routes.map((r) => routeKey(r.source, r.destination)));
}

export function clearCctpExecutableRoutesForTests(): void {
  cctpExecutableRoutes = new Set();
}

export function routeKey(
  source: ChainFamily,
  destination: ChainFamily
): string {
  return `${source}->${destination}`;
}

export function hasBackendRoute(
  source: ChainFamily,
  destination: ChainFamily
): boolean {
  if (source === destination && source === 'stellar') {
    return true;
  }
  if (cctpExecutableRoutes.has(routeKey(source, destination))) {
    return true;
  }
  return CROSS_CHAIN_BACKEND_ROUTES.has(routeKey(source, destination));
}

export function chainSigningSupport(
  chainFamily: ChainFamily,
  opts?: {
    connected?: boolean;
    networkMatch?: boolean;
    canSign?: boolean;
  }
): ExecutionSupport {
  if (!opts?.connected) {
    return {
      kind: 'unsupported',
      code: 'not_connected',
      message: `Connect a ${chainFamily} wallet to sign on-chain messages or transactions.`,
      resolution: 'Install and connect a supported browser wallet',
    };
  }

  if (opts.networkMatch === false) {
    return {
      kind: 'degraded',
      code: 'network_mismatch',
      message:
        'Wallet network does not match the app. Signing is blocked until networks align.',
      resolution: 'Switch the wallet network to match the app',
    };
  }

  if (opts.canSign === false) {
    return {
      kind: 'degraded',
      code: 'wallet_capability_missing',
      message: `Connected wallet cannot sign on ${chainFamily}.`,
      resolution: 'Allow signing in wallet settings or use another wallet',
    };
  }

  return {
    kind: 'signing_only',
    code: 'chain_signing_available',
    message: `${chainFamily} wallet signing is available. Cross-chain swaps are not executable until backend routes exist.`,
    resolution:
      'You can connect and sign chain-native payloads; swaps remain Stellar-only for now',
  };
}

export function resolveExecutionSupport(
  adapterChain: ChainFamily,
  routeHint?: {
    sourceChain?: ChainFamily;
    destinationChain?: ChainFamily;
  },
  signing?: {
    connected?: boolean;
    networkMatch?: boolean;
    canSign?: boolean;
  }
): ExecutionSupport {
  const source = routeHint?.sourceChain ?? adapterChain;
  const destination = routeHint?.destinationChain ?? adapterChain;

  if (source === 'stellar' && destination === 'stellar') {
    // Evaluate live session state before claiming the native swap path.
    const signingStatus = chainSigningSupport('stellar', signing);
    if (
      signingStatus.kind === 'degraded' ||
      signingStatus.code === 'not_connected'
    ) {
      return signingStatus;
    }
    return {
      kind: 'supported',
      code: 'stellar_native',
      message:
        'Stellar native swap path is supported when a Stellar wallet is connected.',
    };
  }

  if (!hasBackendRoute(source, destination)) {
    const signingStatus = chainSigningSupport(adapterChain, signing);
    // Prefer live wallet problems (mismatch / not connected / missing caps)
    // over the generic "no route" message when both apply.
    if (
      signingStatus.kind === 'degraded' ||
      signingStatus.code === 'not_connected'
    ) {
      return signingStatus;
    }
    return {
      kind: 'unsupported',
      code: 'no_backend_route',
      message: `No backend route for ${source} → ${destination}. Wallet may still sign native ${adapterChain} payloads, but swaps cannot execute.`,
      resolution:
        'Use Stellar pairs for swaps, or wait for cross-chain routing support',
    };
  }

  return chainSigningSupport(adapterChain, signing);
}

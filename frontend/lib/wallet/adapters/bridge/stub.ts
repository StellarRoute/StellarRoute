import { WalletAdapterError } from '../errors';
import type { ExecutionSupport } from '../types';
import type {
  BridgeExecutionProvider,
  BridgePrepareRequest,
  BridgePreparedPayload,
  BridgeQuote,
  BridgeQuoteRequest,
  BridgeRouteHint,
  BridgeSubmitRequest,
  BridgeSubmitResult,
} from './types';

function unavailable(route: BridgeRouteHint): ExecutionSupport {
  return {
    kind: 'unsupported',
    code: 'no_backend_route',
    message: `Bridge provider has no backend route for ${route.sourceChain} → ${route.destinationChain}.`,
    resolution:
      'Cross-chain execution is not wired yet; Stellar native swaps remain available',
  };
}

/**
 * Placeholder bridge provider — keeps the interface stable for UI/SDK
 * wiring while backend routes are absent.
 */
export function createStubBridgeProvider(
  id = 'bridge-stub',
  label = 'Bridge (not implemented)'
): BridgeExecutionProvider {
  return {
    id,
    label,
    getAvailability(route) {
      return unavailable(route);
    },
    async quote(request: BridgeQuoteRequest): Promise<BridgeQuote> {
      throw new WalletAdapterError(
        `Bridge quote not implemented for ${request.sourceChain} → ${request.destinationChain}`,
        'unsupported_capability',
        id
      );
    },
    async prepare(request: BridgePrepareRequest): Promise<BridgePreparedPayload> {
      void request;
      throw new WalletAdapterError(
        'Bridge prepare not implemented',
        'unsupported_capability',
        id
      );
    },
    async submit(request: BridgeSubmitRequest): Promise<BridgeSubmitResult> {
      void request;
      return {
        status: 'not_implemented',
        message: 'Bridge submit is not available until backend routes exist',
      };
    },
  };
}

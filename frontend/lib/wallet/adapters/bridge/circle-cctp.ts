import { WalletAdapterError } from '../errors';
import type { ExecutionSupport } from '../types';
import { findExecutableCorridor } from '@/lib/cctp/readiness';
import { getCctpApiClient } from '@/lib/cctp/client';
import { buildCctpQuoteRequest } from '@/lib/cctp/corridor-bridge';
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

function corridorKey(route: BridgeRouteHint): string | null {
  const map: Record<string, string> = {
    stellar: 'stellar:testnet',
    evm: 'eip155:11155111',
  };
  const source = map[route.sourceChain as string];
  const dest = map[route.destinationChain as string];
  if (!source || !dest) return null;
  return `${source}|${dest}`;
}

function notExecutable(route: BridgeRouteHint): ExecutionSupport {
  return {
    kind: 'unsupported',
    code: 'no_backend_route',
    message: `Circle CCTP is not executable for ${route.sourceChain} → ${route.destinationChain} until backend lists the corridor.`,
    resolution: 'Wait for /api/v2 readiness to mark this corridor executable',
  };
}

/** Circle CCTP provider backed by the approved `/api/v2/bridge/cctp/*` contract. */
export function createCircleCctpBridgeProvider(
  id = 'circle-cctp',
  label = 'Circle CCTP',
  client = getCctpApiClient(),
): BridgeExecutionProvider {
  return {
    id,
    label,
    getAvailability(route) {
      const key = corridorKey(route);
      if (!key) return notExecutable(route);
      const [source, dest] = key.split('|');
      const corridor = findExecutableCorridor(source, dest);
      if (!corridor?.executable) {
        return notExecutable(route);
      }
      return {
        kind: 'supported',
        code: 'chain_signing_available',
        message: 'Circle CCTP corridor is executable on this API.',
      };
    },
    async quote(request: BridgeQuoteRequest): Promise<BridgeQuote> {
      const key = corridorKey(request);
      if (!key) {
        throw new WalletAdapterError(
          'Unsupported CCTP route',
          'unsupported_capability',
          id,
        );
      }
      const [sourceCaip, destCaip] = key.split('|');
      const direction =
        sourceCaip.startsWith('stellar') ? 'stellar_to_evm' : 'evm_to_stellar';
      const body = buildCctpQuoteRequest({
        sourceChainId:
          direction === 'stellar_to_evm' ? 'stellar' : 'ethereum-sepolia',
        destChainId:
          direction === 'stellar_to_evm' ? 'ethereum-sepolia' : 'stellar',
        amount: request.amountIn,
        recipient: request.recipient ?? '',
        sender: request.sender,
      });
      if (!body) {
        throw new WalletAdapterError(
          'Invalid CCTP quote request',
          'unsupported_capability',
          id,
        );
      }
      const response = await client.quote(body, {
        idempotencyKey: crypto.randomUUID(),
      });
      return {
        quoteId: response.transfer_id,
        amountOut: response.destination_amount,
        expiresAt: new Date(response.expires_at * 1000).toISOString(),
        provider: id,
        route: request,
        meta: { corridor_id: response.corridor_id, finality: response.finality },
      };
    },
    async prepare(request: BridgePrepareRequest): Promise<BridgePreparedPayload> {
      void request;
      throw new WalletAdapterError(
        'Use the CCTP saga hook with session access token for prepare',
        'unsupported_capability',
        id,
      );
    },
    async submit(request: BridgeSubmitRequest): Promise<BridgeSubmitResult> {
      void request;
      return {
        status: 'not_implemented',
        message: 'CCTP submit is hash-only via saga orchestration',
      };
    },
  };
}

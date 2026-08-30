/**
 * Bridge / cross-chain execution provider interfaces.
 *
 * Backend prepare/submit routes for non-Stellar legs do not exist yet.
 * Callers may depend on these types, but concrete providers must return
 * `available: false` (or throw `not_implemented`) until routes land.
 *
 * Never custody keys here — providers orchestrate wallet adapters + API.
 */

import type {
  ChainFamily,
  ChainWalletSession,
  ExecutionSupport,
} from '../types';

export type BridgeRouteHint = {
  sourceChain: ChainFamily;
  destinationChain: ChainFamily;
  /** Optional CAIP-19 / chain-scoped asset ids once routing exposes them. */
  sourceAsset?: string;
  destinationAsset?: string;
};

export type BridgeQuoteRequest = BridgeRouteHint & {
  amountIn: string;
  /** Slippage in basis points when the backend accepts it. */
  slippageBps?: number;
  sender?: string;
  recipient?: string;
};

export type BridgeQuote = {
  quoteId: string;
  amountOut: string;
  expiresAt?: string;
  provider: string;
  route: BridgeRouteHint;
  /** Opaque provider metadata — never secrets. */
  meta?: Record<string, unknown>;
};

export type BridgePrepareRequest = {
  quoteId: string;
  session: ChainWalletSession;
};

/**
 * Prepared unsigned payload for the source (or destination) chain wallet.
 * Shape is intentionally opaque until backend contracts stabilize.
 */
export type BridgePreparedPayload = {
  prepareId: string;
  chainFamily: ChainFamily;
  /** Discriminated later by adapters (`evm_transaction`, etc.). */
  payloadKind: string;
  payload: unknown;
  expiresAt?: string;
};

export type BridgeSubmitRequest = {
  prepareId: string;
  /** Signed payload returned by a ChainWalletAdapter. */
  signedPayload: unknown;
};

export type BridgeSubmitResult = {
  status: 'submitted' | 'pending' | 'failed' | 'not_implemented';
  txHash?: string;
  message?: string;
};

export type BridgeExecutionProvider = {
  readonly id: string;
  readonly label: string;

  /** Whether this provider can serve a route today. */
  getAvailability(route: BridgeRouteHint): ExecutionSupport;

  quote(request: BridgeQuoteRequest): Promise<BridgeQuote>;

  prepare(request: BridgePrepareRequest): Promise<BridgePreparedPayload>;

  submit(request: BridgeSubmitRequest): Promise<BridgeSubmitResult>;
};

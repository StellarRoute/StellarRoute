/**
 * Stellar asset descriptor returned by the API.
 */
export interface Asset {
  /** Stellar asset type. */
  asset_type: 'native' | 'credit_alphanum4' | 'credit_alphanum12';
  /** Asset code, e.g. `"USDC"`. Absent for native XLM. */
  asset_code?: string;
  /** G-address of the issuing account. Absent for native XLM. */
  asset_issuer?: string;
}

/**
 * Chain-scoped asset used by the `/api/v2` seam.
 * Distinct chains never collide on human symbols like `"USDC"`.
 *
 * Wire form is CAIP-inspired. Solana/TRON chain ids use internal network labels
 * (not genesis-hash CAIP-2). Natives use numeric SLIP-44 (never `slip44:native`).
 */
export interface ChainAsset {
  /** Chain id, e.g. `"stellar:pubnet"`, `"eip155:1"`. */
  chain_id: string;
  /** Asset suffix, e.g. `"slip44:148"`, `"erc20:0x…"`. */
  asset: string;
  /** Full canonical id (`{chain_id}/{asset}`). */
  canonical: string;
  /** Optional human symbol (not unique across chains). */
  symbol?: string;
}

/** Supported chain namespaces for the chain-aware foundation. */
export type ChainNamespace = 'stellar' | 'eip155' | 'solana' | 'bip122' | 'tron';

/**
 * Bridge / cross-chain venue metadata (abstraction only — no settlement).
 */
export interface BridgeVenueMeta {
  provider: string;
  source_chain: string;
  destination_chain: string;
}

/**
 * Response from `GET /api/v2` (`data` payload inside the API envelope).
 *
 * Seam surface today: info + `POST /api/v2/assets/canonicalize` only.
 * There is no v2 quote client method — call those HTTP paths directly.
 */
export interface ApiV2Info {
  version: number;
  chain_aware_assets: boolean;
  bridge_venues_metadata_only: boolean;
  /** Always false until settlement exists. */
  bridge_settlement_executable: boolean;
  supported_chain_namespaces: ChainNamespace[];
  /** Advertised CCTP corridors (empty until backend health gates execution). */
  supported_corridors: SupportedCorridor[];
}

/** Advertised CCTP corridor capability (metadata; may be non-executable). */
export interface SupportedCorridor {
  corridor_id: string;
  provider: string;
  direction: CctpDirection;
  source_chain_id: string;
  destination_chain_id: string;
  source_asset: ChainAsset;
  destination_asset: ChainAsset;
  executable: boolean;
}

export type CctpDirection = 'stellar_to_evm' | 'evm_to_stellar';
export type CctpFinality = 'standard' | 'fast';

export type CctpTransferStatus =
  | 'created'
  | 'burn_prepared'
  | 'burn_submitted'
  | 'awaiting_attestation'
  | 'attestation_ready'
  | 'mint_prepared'
  | 'mint_submitted'
  | 'completed'
  | 'attestation_failed'
  | 'mint_failed_retryable'
  | 'cancelled'
  | 'provider_killed';

export interface CctpFeeQuote {
  source_fee?: string;
  destination_fee?: string;
  bridge_fee?: string;
  fee_asset?: ChainAsset;
}

export type PreparedWalletPayload =
  | {
      type: 'stellar_xdr';
      network_passphrase: string;
      xdr_envelope: string;
      /** Optional signing account (G) — set for trustline ChangeTrust payloads. */
      source?: string;
    }
  | {
      type: 'evm_transaction';
      chain_id: string;
      to: string;
      data: string;
      value: string;
    };

export interface CctpStatusDetails {
  code: string;
  message: string;
  retryable?: boolean;
}

export interface CctpQuoteRequest {
  corridor_id: string;
  provider: string;
  direction: CctpDirection;
  source_chain_id: string;
  destination_chain_id: string;
  source_asset: ChainAsset;
  destination_asset: ChainAsset;
  /** Decimal string; never a float. */
  amount: string;
  /**
   * Destination recipient. `stellar_to_evm`: EVM `0x` address.
   * `evm_to_stellar`: Stellar account G-address only (no muxed M or contract C strkeys).
   */
  recipient: string;
  /**
   * Optional source sender. Stellar burn: G-address only. EVM burn: `0x` address.
   * Invalid sender returns `validation_error` (HTTP 400).
   */
  sender?: string;
  /** Required for `evm_to_stellar` — Stellar G-address fee-payer for mint preparation. */
  mint_submitter?: string;
  finality: CctpFinality;
}

export interface CctpQuoteResponse {
  transfer_id: string;
  corridor_id: string;
  provider: string;
  direction: CctpDirection;
  source_amount: string;
  destination_amount: string;
  fee_quote: CctpFeeQuote;
  expires_at: number;
  finality: CctpFinality;
  /** One-time bearer token for transfer mutations/status (store securely). */
  access_token: string;
}

/** Optional auth/idempotency headers for CCTP transfer calls. */
export interface CctpCallOptions {
  accessToken?: string;
  idempotencyKey?: string;
  signal?: AbortSignal;
}

export const CCTP_TRANSFER_ACCESS_HEADER = 'x-cctp-transfer-access';
export const CCTP_IDEMPOTENCY_HEADER = 'idempotency-key';

export interface CctpTransferStatusResponse {
  transfer_id: string;
  corridor_id: string;
  provider: string;
  direction: CctpDirection;
  status: CctpTransferStatus;
  source_tx_hash?: string;
  destination_tx_hash?: string;
  support_reference_id?: string;
  retryable: boolean;
  error?: CctpStatusDetails;
  /** Unix seconds (UTC) until re-attest may be requested again. */
  reattest_cooldown_until?: number;
}

export interface CctpPrepareBurnResponse {
  transfer_id: string;
  status: CctpTransferStatus;
  payload: PreparedWalletPayload;
  expires_at: number;
  approval_required?: boolean;
}

export interface CctpSubmitBurnRequest {
  tx_hash: string;
}

export interface CctpSubmitBurnResponse {
  transfer_id: string;
  status: CctpTransferStatus;
  source_tx_hash: string;
}

export interface CctpPrepareMintResponse {
  transfer_id: string;
  status: CctpTransferStatus;
  payload: PreparedWalletPayload;
  expires_at: number;
  /** True when wallet must submit USDC ChangeTrust before mint_and_forward. */
  trustline_required?: boolean;
}

export interface CctpSubmitMintRequest {
  tx_hash: string;
}

export interface CctpSubmitMintResponse {
  transfer_id: string;
  status: CctpTransferStatus;
  destination_tx_hash: string;
}

export interface CctpReattestResponse {
  transfer_id: string;
  status: CctpTransferStatus;
  retryable: boolean;
}

/** Documented testnet corridor id (metadata only). */
export const CCTP_TESTNET_CORRIDOR_ID =
  'circle-cctp:usdc:stellar-testnet:ethereum-sepolia';

/** Circle CCTP provider id. */
export const CCTP_PROVIDER_ID = 'circle-cctp';

/** Response from `POST /api/v2/assets/canonicalize`. */
export interface CanonicalizeAssetResponse {
  asset: ChainAsset;
  input_form: 'legacy_stellar' | 'caip19' | string;
}

const CAIP_PREFIX = /^(stellar|eip155|solana|bip122|tron):/i;
const STELLAR_ISSUER_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
const BASE58_ALPHABET =
  '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

const SLIP44: Record<string, number> = {
  stellar: 148,
  eip155: 60,
  solana: 501,
  bip122: 0,
  tron: 195,
};

/** Returns true when `input` looks like a chain-scoped identifier. */
export function looksLikeCaip(input: string): boolean {
  return CAIP_PREFIX.test(input.trim());
}

function fail(msg: string): never {
  throw new Error(msg);
}

function validateStellarIssuer(issuer: string): string {
  if (issuer.length !== 56 || !issuer.startsWith('G')) {
    fail(`invalid stellar issuer (expected G… length 56): ${issuer}`);
  }
  for (const c of issuer.slice(1)) {
    if (!STELLAR_ISSUER_ALPHABET.includes(c)) {
      fail(`invalid stellar issuer alphabet: ${issuer}`);
    }
  }
  return issuer;
}

function normalizeStellarCode(code: string): string {
  if (
    !code ||
    code.length > 12 ||
    ![...code].every((c) => /[A-Za-z0-9]/.test(c))
  ) {
    fail(`invalid stellar asset code: ${code}`);
  }
  return code.toUpperCase();
}

function validateErc20(address: string): string {
  const lower = address.toLowerCase();
  if (!/^0x[a-f0-9]{40}$/.test(lower)) {
    fail(`invalid erc20 address (expected 0x + 40 hex): ${address}`);
  }
  return lower;
}

function validateSolana(address: string): string {
  if (address.length < 32 || address.length > 44) {
    fail(`invalid solana address length: ${address}`);
  }
  for (const c of address) {
    if (!BASE58_ALPHABET.includes(c)) {
      fail(`invalid solana address alphabet: ${address}`);
    }
  }
  return address;
}

function validateTron(address: string): string {
  if (address.length !== 34 || !address.startsWith('T')) {
    fail(`invalid tron address (expected T… length 34): ${address}`);
  }
  for (const c of address) {
    if (!BASE58_ALPHABET.includes(c)) {
      fail(`invalid tron address alphabet: ${address}`);
    }
  }
  return address;
}

function parseChainId(caip2: string): { ns: string; ref: string; id: string } {
  const idx = caip2.indexOf(':');
  if (idx < 0) fail(`invalid chain id: ${caip2}`);
  const ns = caip2.slice(0, idx).toLowerCase();
  const ref = caip2.slice(idx + 1);
  if (ns === 'stellar') {
    const network = ref.toLowerCase();
    if (network !== 'pubnet' && network !== 'testnet') {
      fail(`unsupported stellar network: ${ref}`);
    }
    return { ns, ref: network, id: `stellar:${network}` };
  }
  if (ns === 'eip155') {
    if (!/^\d+$/.test(ref)) fail(`invalid eip155 chain id: ${ref}`);
    return { ns, ref, id: `eip155:${ref}` };
  }
  if (ns === 'solana') {
    const cluster = ref.toLowerCase();
    if (!['mainnet', 'devnet', 'testnet'].includes(cluster)) {
      fail(`unsupported solana cluster label: ${ref}`);
    }
    return { ns, ref: cluster, id: `solana:${cluster}` };
  }
  if (ns === 'bip122') {
    if (!/^[a-fA-F0-9]{64}$/.test(ref)) {
      fail(`invalid bip122 genesis hash (expected 64 hex chars): ${ref}`);
    }
    const genesis = ref.toLowerCase();
    return { ns, ref: genesis, id: `bip122:${genesis}` };
  }
  if (ns === 'tron') {
    const network = ref.toLowerCase();
    if (!['mainnet', 'nile', 'shasta'].includes(network)) {
      fail(`unsupported tron network label: ${ref}`);
    }
    return { ns, ref: network, id: `tron:${network}` };
  }
  fail(`unsupported chain namespace: ${ns}`);
}

/**
 * Canonicalize a chain-scoped or legacy Stellar asset id.
 * Matches Rust `canonicalize_asset_id` byte-for-byte for fixture vectors.
 * Throws on malformed chain-scoped ids (fail closed — never echoes invalid input).
 *
 * Prefer `POST /api/v2/assets/canonicalize` for authoritative server parsing.
 */
export function canonicalizeAssetId(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) fail('asset identifier is empty');

  if (looksLikeCaip(trimmed)) {
    const slash = trimmed.indexOf('/');
    if (slash < 0) fail(`expected chain/asset form, got: ${trimmed}`);
    const chain = parseChainId(trimmed.slice(0, slash));
    const assetPart = trimmed.slice(slash + 1);
    if (
      assetPart.toLowerCase() === 'native' ||
      assetPart.toLowerCase() === 'slip44:native'
    ) {
      fail(
        "slip44:native / bare native are not allowed; use slip44:<coin_type>",
      );
    }
    const colon = assetPart.indexOf(':');
    if (colon < 0) {
      fail(`expected asset_namespace:reference, got: ${assetPart}`);
    }
    const assetNs = assetPart.slice(0, colon);
    const assetRef = assetPart.slice(colon + 1);

    if (assetNs === 'slip44') {
      const slip = Number(assetRef);
      const expected = SLIP44[chain.ns];
      if (!Number.isInteger(slip) || slip !== expected) {
        fail(
          `slip44:${assetRef} is not valid for chain ${chain.id} (expected slip44:${expected})`,
        );
      }
      return `${chain.id}/slip44:${expected}`;
    }
    if (assetNs === 'stellar') {
      if (chain.ns !== 'stellar') {
        fail('stellar credit assets require a stellar:* chain id');
      }
      const split = assetRef.indexOf(':');
      if (split < 0) fail(`stellar asset requires CODE:ISSUER, got: ${assetRef}`);
      const code = normalizeStellarCode(assetRef.slice(0, split));
      const issuer = validateStellarIssuer(assetRef.slice(split + 1));
      return `${chain.id}/stellar:${code}:${issuer}`;
    }
    if (assetNs === 'erc20') {
      if (chain.ns !== 'eip155') fail('erc20 assets require an eip155:* chain id');
      return `${chain.id}/erc20:${validateErc20(assetRef)}`;
    }
    if (assetNs === 'token') {
      if (chain.ns !== 'solana') fail('solana token assets require a solana:* chain id');
      return `${chain.id}/token:${validateSolana(assetRef)}`;
    }
    if (assetNs === 'trc20') {
      if (chain.ns !== 'tron') fail('trc20 assets require a tron:* chain id');
      return `${chain.id}/trc20:${validateTron(assetRef)}`;
    }
    fail(`unsupported asset namespace: ${assetNs}`);
  }

  const lower = trimmed.toLowerCase();
  if (lower === 'xlm' || lower === 'native') {
    return 'stellar:pubnet/slip44:148';
  }
  if (trimmed.includes(':')) {
    const [code, issuer] = trimmed.split(':');
    return `stellar:pubnet/stellar:${normalizeStellarCode(code)}:${validateStellarIssuer(issuer)}`;
  }
  fail(`unrecognized stellar legacy asset: ${trimmed}`);
}

/**
 * A single tradeable asset pair with active orderbook depth.
 */
export interface TradingPair {
  /** Human-readable base asset code, e.g. `"XLM"`. */
  base: string;
  /** Human-readable counter asset code, e.g. `"USDC"`. */
  counter: string;
  /** Canonical base asset identifier (`"native"` or `"CODE:ISSUER"`). */
  base_asset: string;
  /** Canonical counter asset identifier. */
  counter_asset: string;
  /** Number of active offers for this pair. */
  offer_count: number;
  /** RFC-3339 timestamp of the most recent offer update. */
  last_updated?: string;
}

/**
 * Response from `GET /api/v1/pairs`.
 */
export interface PairsResponse {
  /** Active trading pairs ordered by liquidity depth. */
  pairs: TradingPair[];
  /** Total number of pairs returned. */
  total: number;
}

/**
 * A single price level in the orderbook.
 */
export interface OrderbookEntry {
  /** Price as a decimal string (7 decimal places). */
  price: string;
  /** Available amount at this price level. */
  amount: string;
  /** Total value at this price level (`price × amount`). */
  total: string;
}

/**
 * Full orderbook snapshot for a trading pair.
 * Response from `GET /api/v1/orderbook/{base}/{quote}`.
 */
export interface Orderbook {
  base_asset: Asset;
  quote_asset: Asset;
  /** Buy orders sorted highest price first. */
  bids: OrderbookEntry[];
  /** Sell orders sorted lowest price first. */
  asks: OrderbookEntry[];
  /** Unix timestamp of the snapshot. */
  timestamp: number;
}

/**
 * Direction of a price quote.
 * - `"sell"` — how much quote asset you receive when selling `amount` of the base asset.
 * - `"buy"`  — how much base asset you must spend to buy `amount` of the quote asset.
 */
export type QuoteType = 'sell' | 'buy';

/**
 * A single hop in the optimal execution path.
 */
export interface PathStep {
  from_asset: Asset;
  to_asset: Asset;
  /** Exchange rate for this hop. */
  price: string;
  /** Liquidity source: `"sdex"` or `"amm:<pool_address>"`. */
  source: string;
  /** Total liquidity depth available at this hop's price */
  liquidity_depth?: string;
  /** Fee in basis points for this hop (e.g., 30 for 0.3%) */
  fee_bps?: number;
}

/**
 * Best available price quote with full routing path.
 * Response from `GET /api/v1/quote/{base}/{quote}`.
 */
export interface PriceQuote {
  base_asset: Asset;
  quote_asset: Asset;
  /** Input amount that was quoted. */
  amount: string;
  /** Effective price (quote asset per base asset unit). */
  price: string;
  /** Total output amount (`amount × price`). */
  total: string;
  /** Direction of the quote. */
  quote_type: QuoteType;
  /** Ordered list of hops in the optimal execution path. */
  path: PathStep[];
  /** Unix timestamp when the quote was generated. */
  timestamp: number;
  /** Unix timestamp (ms) when this quote expires and should be considered stale */
  expires_at?: number;
  /** Unix timestamp (ms) of the underlying data source (e.g., orderbook snapshot) */
  source_timestamp?: number;
  /** Time-to-live in seconds for client-side staleness detection */
  ttl_seconds?: number;
  /** Estimated price impact percentage */
  price_impact?: string;
  /** Rationale for quote venue selection. */
  rationale?: {
    /** The selection strategy used (e.g., "highest_liquidity", "best_price") */
    strategy: string;
    /** Comparison across different liquidity venues */
    compared_venues: Array<{
      /** Source identifier (e.g., "sdex", "amm:...") */
      source: string;
      /** Quote price from this source */
      price: string;
      /** Total depth available at this price */
      available_amount: string;
      /** Whether the quote was considered executable */
      executable: boolean;
    }>;
  };
}

/**
 * A single request item for a batch quote.
 */
export interface QuoteRequestItem {
  /** Base asset identifier: "native", "CODE", or "CODE:ISSUER". */
  base: string;
  /** Quote asset identifier. */
  quote: string;
  /** Amount to trade (optional). */
  amount?: number;
  /** Direction of the quote ("sell" or "buy"). Defaults to "sell". */
  quote_type?: QuoteType;
}

/**
 * Response from a batch quote request.
 */
export interface BatchQuoteResponse {
  /** Array of quotes in the same order as requested. */
  quotes: PriceQuote[];
  /** Total number of quotes successfully fetched. */
  total: number;
}

/**
 * A single request item for a batch orderbook lookup.
 */
export interface OrderbookRequestItem {
  /** Base asset identifier: "native", "CODE", or "CODE:ISSUER". */
  base: string;
  /** Quote asset identifier. */
  quote: string;
}

/**
 * Per-item error returned inside a batch response.
 */
export interface BatchItemError {
  /** Machine-readable error code. */
  code: string;
  /** Human-readable description. */
  message: string;
}

/**
 * Result for a single item in a batch orderbook response.
 */
export interface BatchOrderbookItemResult {
  /** Zero-based index of this item in the original request. */
  index: number;
  /** The orderbook, present when `status === "ok"`. */
  orderbook?: Orderbook;
  /** Per-item error, present when `status === "error"`. */
  error?: BatchItemError;
  /** `"ok"` or `"error"`. */
  status: string;
}

/**
 * Response from a batch orderbook request.
 */
export interface BatchOrderbookResponse {
  /** Results in the same order as the request items. */
  results: BatchOrderbookItemResult[];
  /** Number of items that succeeded. */
  items_succeeded: number;
  /** Number of items that failed (per-item errors, not a batch-level failure). */
  items_failed: number;
  /** Total items in the batch. */
  total: number;
}

/**
 * Configuration for quote staleness detection
 */
export interface QuoteStalenessConfig {
  /** Maximum quote age in seconds before considering stale (default: 30) */
  max_age_seconds: number;
  /** Whether to reject stale quotes on the client side */
  reject_stale?: boolean;
}

/**
 * Default staleness configuration
 */
export const DEFAULT_STALENESS_CONFIG: QuoteStalenessConfig = {
  max_age_seconds: 30,
  reject_stale: false,
};

/**
 * Check if a quote is considered stale
 */
export function isQuoteStale(quote: PriceQuote, config: QuoteStalenessConfig = DEFAULT_STALENESS_CONFIG): boolean {
  const now = Date.now();
  const ageMs = now - quote.timestamp;
  const maxAgeMs = config.max_age_seconds * 1000;
  return ageMs > maxAgeMs;
}

/**
 * Check if a quote has expired based on its expires_at field
 */
export function isQuoteExpired(quote: PriceQuote): boolean {
  if (!quote.expires_at) return false;
  return Date.now() > quote.expires_at;
}

/**
 * Get remaining time until quote expires (in seconds), or null if no expiry
 */
export function getTimeUntilExpiry(quote: PriceQuote): number | null {
  if (!quote.expires_at) return null;
  const remaining = quote.expires_at - Date.now();
  return remaining > 0 ? Math.floor(remaining / 1000) : 0;
}

/**
 * Service health check result.
 * Response from `GET /health`.
 */
export interface HealthStatus {
  /** Overall service status. */
  status: 'healthy' | 'unhealthy';
  /** Deployed package version string. */
  version: string;
  /** ISO-8601 UTC timestamp of the health check. */
  timestamp: string;
  /** Per-dependency health map, e.g. `{ database: "healthy" }`. */
  components: Record<string, string>;
}

/**
 * Optimal trading route without pricing details.
 * Response from `GET /api/v1/route/{base}/{quote}`.
 */
export interface RouteResponse {
  base_asset: Asset;
  quote_asset: Asset;
  /** Input amount being traded. */
  amount: string;
  /** Execution steps for this trade. */
  path: PathStep[];
  /** Slippage tolerance in basis points. */
  slippage_bps: number;
  /** Unix timestamp of the route calculation. */
  timestamp: number;
}

// ── Route simulation (dry-run) ───────────────────────────────────────────────

/**
 * A single hop in a route simulation dry-run request.
 */
export interface SimulationHop {
  /** Source asset identifier: `"native"`, `"CODE"`, or `"CODE:ISSUER"`. */
  from_asset: string;
  /** Destination asset identifier. */
  to_asset: string;
  /** Liquidity source: `"sdex"` or `"amm:<pool_address>"`. */
  source: string;
  /** Fee in basis points for this hop. */
  fee_bps?: number;
  /** Optional price hint for diagnostics. */
  price?: string;
  /** Venue reference for slippage override lookup. */
  venue_ref?: string;
}

/**
 * Per-hop slippage override for simulation.
 */
export interface SimulationSlippageOverride {
  /** Which venue to apply the override to. */
  venue_ref: string;
  /** Slippage tolerance in basis points. */
  slippage_bps: number;
}

/**
 * Request body for `POST /api/v1/simulate/route`.
 */
export interface SimulateRouteRequest {
  /** Route to simulate, containing execution-order hops. */
  route: { hops: SimulationHop[] };
  /** Input amount for the simulation. */
  amount: string;
  /** Default slippage tolerance in basis points (default: 50). */
  slippage_bps?: number;
  /** Per-hop slippage overrides applied by venue_ref. */
  slippage_bps_overrides?: SimulationSlippageOverride[];
}

/**
 * Reason a venue was excluded during simulation.
 */
export type ExclusionReason =
  | 'policy_threshold'
  | 'override'
  | 'stale_data'
  | 'circuit_breaker_open'
  | 'liquidity_anomaly'
  | (string & Record<never, never>);

/**
 * Information about an excluded venue.
 */
export interface ExcludedVenueInfo {
  venue_ref: string;
  reason: ExclusionReason;
}

/**
 * Diagnostics about venues excluded during simulation.
 */
export interface ExclusionDiagnostics {
  excluded_venues: ExcludedVenueInfo[];
}

/**
 * Response from `POST /api/v1/simulate/route`.
 */
export interface SimulateRouteResponse {
  /** The simulated quote with full path and pricing details. */
  quote: PriceQuote;
  /** Optional diagnostics about venues excluded during simulation. */
  exclusion_diagnostics?: ExclusionDiagnostics;
}

/**
 * A single hop within a ranked route candidate.
 */
export interface RankedRouteHop {
  from_asset: Asset;
  to_asset: Asset;
  /** Exchange rate for this hop. */
  price: string;
  /** Amount received after this hop. */
  amount_out_of_hop: string;
  /** Fee in basis points for this hop. */
  fee_bps: number;
  /** Liquidity source: `"sdex"` or `"amm:<pool_address>"`. */
  source: string;
}

/**
 * A single ranked route candidate returned by `/api/v1/routes`.
 */
export interface RankedRouteCandidate {
  /** Final output amount after all hops. */
  estimated_output: string;
  /** Price impact in basis points. */
  impact_bps: number;
  /** Composite ranking score — higher is better. */
  score: number;
  /** Optimizer policy used, e.g. `"production"`. */
  policy_used: string;
  /** Ordered list of hops for this route. */
  path: RankedRouteHop[];
}

/**
 * Response from `GET /api/v1/routes/{base}/{quote}`.
 */
export interface RankedRoutesResponse {
  base_asset: Asset;
  quote_asset: Asset;
  /** Input amount that was quoted. */
  amount: string;
  /** Ranked route candidates, ordered by composite score descending. */
  routes: RankedRouteCandidate[];
  /** Unix timestamp of the route calculation. */
  timestamp: number;
}

/**
 * Supported time windows for price history queries.
 */
export type PriceHistoryWindow = '1h' | '4h' | '24h' | '7d' | '30d';

/**
 * A single price history data point.
 */
export interface PriceHistoryPoint {
  /** Unix timestamp (ms) of this data point (hour-truncated). */
  timestamp: number;
  /** Average mid-price for this interval. */
  price: string;
}

/**
 * Response from `GET /api/v1/price-history/{base}/{quote}`.
 */
export interface PriceHistoryResponse {
  base_asset: Asset;
  quote_asset: Asset;
  /** Time window of the returned data. */
  window: string;
  /** Data source identifier. */
  source: string;
  /** Unix timestamp (ms) when this response was generated. */
  generated_at: number;
  /** Ordered price history points (oldest first). */
  points: PriceHistoryPoint[];
}

/** Current prepare execution mode — classic PathPaymentStrictSend only. */
export type SwapExecutionMode = 'classic_path_payment';

/**
 * Request body for `POST /api/v1/swap/prepare`.
 */
export interface SwapPrepareRequest {
  /** Route to prepare (same hop shape as {@link SimulateRouteRequest.route}). */
  route: SimulateRouteRequest['route'];
  /** Input amount as a decimal string. */
  amount: string;
  /** Stellar account G-address of the swap sender. */
  sender: string;
  /** Minimum acceptable output amount as a decimal string (slippage guard). */
  min_output?: string;
  /** Slippage tolerance in basis points (default: 50). */
  slippage_bps?: number;
}

/**
 * Response payload from `POST /api/v1/swap/prepare` (inner `data` field).
 */
export interface PreparedSwapResponse {
  /** Server-issued quote id used for submit idempotency. */
  quote_id: string;
  /** Base64-encoded unsigned Stellar XDR transaction envelope. */
  xdr_envelope: string;
  /** Authoritative expected output from prepare. */
  expected_output: string;
  /** Optional minimum output encoded in the envelope. */
  min_output?: string;
  /** Unix timestamp (ms) after which this prepare quote expires. */
  expires_at: number;
  /** Always `classic_path_payment` on success. */
  execution_mode: SwapExecutionMode | string;
  /** Network passphrase the unsigned envelope was built for (compare before wallet signing). */
  network_passphrase: string;
}

/**
 * Request body for `POST /api/v1/swap/submit`.
 */
export interface SwapSubmitRequest {
  /** Quote id returned by {@link StellarRouteClient.prepareSwap}. */
  quote_id: string;
  /** Base64-encoded signed Stellar XDR transaction envelope. */
  signed_xdr: string;
}

/**
 * Response payload from `POST /api/v1/swap/submit` (inner `data` field).
 */
export interface SwapSubmitResponse {
  /** Quote id that was submitted. */
  quote_id: string;
  /** Horizon transaction hash. */
  tx_hash: string;
  /** Submission status, e.g. `"pending"` or `"success"`. */
  status: string;
  /** Optional observed output amount. */
  output_amount?: string;
  /** Optional ledger number when known. */
  ledger?: number;
}

/**
 * Horizon confirmation result for a submitted swap transaction.
 */
export interface SwapConfirmResult {
  /** Horizon transaction hash. */
  tx_hash: string;
  /** True when Horizon reports `successful: true`. */
  successful: boolean;
  /** Ledger the transaction landed in, when present. */
  ledger?: number;
  /** Horizon transaction detail URL used for confirmation. */
  horizon_url: string;
}

/**
 * Integrator's current Stellar network passphrase, or a callback that returns it
 * (e.g. Freighter `getNetworkDetails().networkPassphrase`).
 */
export type ExecuteSwapNetworkPassphrase =
  | string
  | (() => string | Promise<string>);

/**
 * Parameters for {@link StellarRouteClient.executeSwap}.
 *
 * Orchestrates prepare → network check → caller sign → submit.
 * `signTransaction` and `networkPassphrase` are required.
 */
export interface ExecuteSwapParams {
  /** Route to execute (same shape as SimulateRouteRequest.route). */
  route: SimulateRouteRequest['route'];
  /** Input amount as a decimal string. */
  amount: string;
  /** Stellar account G-address of the swap sender. */
  sender: string;
  /** Minimum acceptable output amount as a decimal string (slippage guard). */
  min_output?: string;
  /** Slippage tolerance in basis points (default: 50). */
  slippage_bps?: number;
  /**
   * Current wallet/app network passphrase (or async getter). Compared to
   * `prepared.network_passphrase` before signing; mismatch returns
   * `network_mismatch` without signing or submitting.
   */
  networkPassphrase: ExecuteSwapNetworkPassphrase;
  /**
   * Signs the unsigned XDR from prepare exactly once.
   * Ambiguous submit retries reuse this same signed envelope.
   */
  signTransaction: (xdrEnvelope: string) => Promise<string>;
  /** Max ambiguous submit retries after the first attempt (default: 2). */
  ambiguousSubmitRetries?: number;
}

/**
 * Aggregated result of {@link StellarRouteClient.executeSwap}.
 */
export interface ExecuteSwapResult {
  /** Quote id from prepare (needed for audit / retries). */
  quote_id: string;
  /** Base64-encoded unsigned Stellar XDR transaction envelope from prepare. */
  xdr_envelope: string;
  /** Authoritative expected output from prepare. */
  expected_output: string;
  /** Optional minimum output from prepare. */
  min_output?: string;
  /** Unix timestamp (ms) after which the prepare quote expires. */
  expires_at: number;
  /** Prepare execution mode (`classic_path_payment`). */
  execution_mode: string;
  /** Network passphrase the unsigned envelope was built for. */
  network_passphrase: string;
  /** Horizon transaction hash from submit. */
  tx_hash: string;
  /** Submission status from submit. */
  status: string;
}

/**
 * Convert a PathStep (quote/route response) into a SimulationHop suitable for
 * prepare/simulate request bodies (legacy Stellar asset strings).
 */
export function pathStepToSimulationHop(step: PathStep): SimulationHop {
  return {
    from_asset: stellarAssetToCanonical(step.from_asset),
    to_asset: stellarAssetToCanonical(step.to_asset),
    source: step.source,
    fee_bps: step.fee_bps,
    price: step.price,
  };
}

/**
 * Legacy Stellar asset identifier: `"native"` or `"CODE:ISSUER"`.
 * Distinct from {@link canonicalizeAssetId} (chain-scoped CAIP form).
 */
export function stellarAssetToCanonical(asset: Asset | string): string {
  if (typeof asset === 'string') return asset;
  if (asset.asset_type === 'native') return 'native';
  const code = asset.asset_code ?? '';
  const issuer = asset.asset_issuer ?? '';
  return issuer ? `${code}:${issuer}` : code;
}

/** @deprecated Use {@link stellarAssetToCanonical}. */
export const assetToCanonical = stellarAssetToCanonical;

/**
 * Error response from the StellarRoute API.
 */
export interface ApiError {
  /** Machine-readable error code, e.g. `"not_found"`. */
  error: string;
  /** Human-readable description. */
  message: string;
  /** Optional structured context (present on validation errors). */
  details?: unknown;
}

/**
 * Canonical error codes documented for the StellarRoute API backend.
 *
 * This array is the single source of truth checked against
 * `crates/api/src/models/response.rs`'s `ApiErrorCode::ALL` and
 * `docs/api/error_taxonomy.md`'s error catalog table by
 * `crates/api/tests/openapi_swap_contract.rs` (issue #1051). Add a new code
 * to all three places together, or that test fails the build.
 */
export const API_ERROR_CODES = [
  'internal_error',
  'bad_request',
  'not_found',
  'validation_error',
  'rate_limit_exceeded',
  'overloaded',
  'unauthorized',
  'invalid_asset',
  'invalid_amount',
  'invalid_slippage',
  'invalid_asset_format',
  'no_route',
  'not_executable',
  'stale_market_data',
  'not_implemented',
  'quote_not_found',
  'quote_expired',
  'duplicate_quote',
  'dependency_unavailable',
  'unsupported_execution_mode',
  'unsupported_route',
  'cctp_not_enabled',
  'unsupported_corridor',
  'invalid_finality',
  'invalid_recipient',
  'fee_quote_unavailable',
  'attestation_pending',
  'attestation_expired',
  'mint_retryable',
  'transfer_not_found',
  'provider_killed',
] as const;

/** A documented backend error code (see {@link API_ERROR_CODES}). */
export type BackendApiErrorCode = (typeof API_ERROR_CODES)[number];

/**
 * Machine-readable error codes returned by the StellarRoute API, plus two
 * codes the SDK itself synthesizes for transport-level failures that never
 * reach the server (`network_error`, `unknown_error`).
 *
 * The trailing `(string & Record<never, never>)` branch keeps this type
 * assignable from arbitrary server strings (forward-compatible with codes
 * added server-side before the SDK updates) while `API_ERROR_CODES` still
 * gives autocomplete and a closed list for the drift check above.
 */
export type ApiErrorCode =
  | BackendApiErrorCode
  | 'network_error' // SDK specific
  | 'network_mismatch' // SDK specific — prepare vs integrator passphrase
  | 'unknown_error' // SDK specific
  | (string & Record<never, never>);

/**
 * CCTP wire types — mirrors `@stellarroute/sdk-js` contract (frontend-local).
 */

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

export interface ChainAsset {
  chain_id: string;
  asset: string;
  canonical: string;
  symbol?: string;
}

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
      source?: string;
      from?: string;
    }
  | {
      type: 'evm_transaction';
      chain_id: string;
      to: string;
      data: string;
      value: string;
      gas?: string;
      gas_price?: string;
      max_fee_per_gas?: string;
      max_priority_fee_per_gas?: string;
      source?: string;
      from?: string;
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
  amount: string;
  recipient: string;
  sender?: string;
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
  access_token: string;
}

export interface CctpCallOptions {
  accessToken?: string;
  idempotencyKey?: string;
  signal?: AbortSignal;
}

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

export interface ApiV2Info {
  version: number;
  chain_aware_assets: boolean;
  bridge_venues_metadata_only: boolean;
  bridge_settlement_executable: boolean;
  supported_chain_namespaces: string[];
  supported_corridors: SupportedCorridor[];
}

export const CCTP_TRANSFER_ACCESS_HEADER = 'x-cctp-transfer-access';
export const CCTP_IDEMPOTENCY_HEADER = 'idempotency-key';

export const CCTP_TESTNET_CORRIDOR_ID =
  'circle-cctp:usdc:stellar-testnet:ethereum-sepolia';
export const CCTP_PROVIDER_ID = 'circle-cctp';

export const STELLAR_TESTNET_CHAIN_ID = 'stellar:testnet';
export const SEPOLIA_CHAIN_ID = 'eip155:11155111';

export const STELLAR_TESTNET_PASSPHRASE =
  'Test SDF Network ; September 2015';

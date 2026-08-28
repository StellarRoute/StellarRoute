import type { PreparedWalletPayload } from './types';

/** Stable fingerprint for comparing fresh server prepare payloads between CTAs. */
export function fingerprintPreparedPayload(payload: PreparedWalletPayload): string {
  if (payload.type === 'stellar_xdr') {
    return `stellar:${payload.network_passphrase}:${payload.xdr_envelope}`;
  }
  return [
    'evm',
    payload.chain_id,
    payload.to.toLowerCase(),
    payload.data,
    payload.value,
    payload.gas ?? '',
    payload.gas_price ?? '',
    payload.max_fee_per_gas ?? '',
    payload.max_priority_fee_per_gas ?? '',
  ].join('|');
}

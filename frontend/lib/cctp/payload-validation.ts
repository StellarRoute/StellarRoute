import type { PreparedWalletPayload } from './types';
import {
  SEPOLIA_CHAIN_ID,
  STELLAR_TESTNET_PASSPHRASE,
} from './types';
import { SEPOLIA_CCTP_CONTRACTS } from './constants';
import { MAX_EVM_CALLDATA_BYTES } from './evm-execution';

export type PayloadValidationResult =
  | { ok: true }
  | { ok: false; code: string; message: string };

const MAX_EVM_GAS = BigInt(2_000_000);
const MAX_EVM_FEE_WEI = BigInt('5000000000000000000'); // 5 ETH upper guard

export function validatePreparedPayload(
  payload: PreparedWalletPayload,
  opts: {
    expectedStellarPassphrase?: string;
    expectedEvmChainId?: string;
    nowSec?: number;
    expiresAtSec?: number;
  } = {},
): PayloadValidationResult {
  const nowSec = opts.nowSec ?? Math.floor(Date.now() / 1000);
  if (opts.expiresAtSec !== undefined && nowSec >= opts.expiresAtSec) {
    return {
      ok: false,
      code: 'payload_expired',
      message: 'Wallet payload expired. Prepare again before signing.',
    };
  }

  if (payload.type === 'stellar_xdr') {
    const expected =
      opts.expectedStellarPassphrase ?? STELLAR_TESTNET_PASSPHRASE;
    if (payload.network_passphrase !== expected) {
      return {
        ok: false,
        code: 'network_mismatch',
        message: 'Prepared Stellar network does not match this app.',
      };
    }
    if (!payload.xdr_envelope?.trim()) {
      return {
        ok: false,
        code: 'validation_error',
        message: 'Prepared Stellar envelope is empty.',
      };
    }
    return { ok: true };
  }

  const expectedChain = opts.expectedEvmChainId ?? SEPOLIA_CHAIN_ID;
  if (payload.chain_id !== expectedChain) {
    return {
      ok: false,
      code: 'network_mismatch',
      message: 'Prepared EVM chain does not match Sepolia testnet.',
    };
  }

  const allowedTo = new Set<string>([
    SEPOLIA_CCTP_CONTRACTS.tokenMessenger,
    SEPOLIA_CCTP_CONTRACTS.messageTransmitter,
    SEPOLIA_CCTP_CONTRACTS.usdc,
  ]);
  if (!allowedTo.has(payload.to.toLowerCase())) {
    return {
      ok: false,
      code: 'validation_error',
      message: 'Prepared EVM target contract is not allowlisted.',
    };
  }

  if (!/^0x[a-fA-F0-9]*$/.test(payload.data)) {
    return {
      ok: false,
      code: 'validation_error',
      message: 'Prepared EVM calldata is malformed.',
    };
  }

  const dataBytes = (payload.data.length - 2) / 2;
  if (dataBytes > MAX_EVM_CALLDATA_BYTES) {
    return {
      ok: false,
      code: 'validation_error',
      message: 'Prepared EVM calldata exceeds allowed length.',
    };
  }

  const valueCheck = validateEvmNumericField(payload.value ?? '0', 'value');
  if (!valueCheck.ok) return valueCheck;
  if ('value' in valueCheck && valueCheck.value > BigInt(0)) {
    return {
      ok: false,
      code: 'validation_error',
      message: 'Prepared EVM transaction must not carry native value.',
    };
  }

  if (payload.gas) {
    const gasCheck = validateEvmNumericField(payload.gas, 'gas');
    if (!gasCheck.ok) return gasCheck;
    if ('value' in gasCheck && gasCheck.value > MAX_EVM_GAS) {
      return {
        ok: false,
        code: 'validation_error',
        message: 'Prepared EVM gas exceeds allowed upper bound.',
      };
    }
  }

  for (const field of [
    payload.gas_price,
    payload.max_fee_per_gas,
    payload.max_priority_fee_per_gas,
  ]) {
    if (!field) continue;
    const feeCheck = validateEvmNumericField(field, 'fee');
    if (!feeCheck.ok) return feeCheck;
    if ('value' in feeCheck && feeCheck.value > MAX_EVM_FEE_WEI) {
      return {
        ok: false,
        code: 'validation_error',
        message: 'Prepared EVM fee fields exceed allowed upper bound.',
      };
    }
  }

  return { ok: true };
}

function validateEvmNumericField(
  raw: string,
  label: string,
): PayloadValidationResult | { ok: true; value: bigint } {
  if (raw.startsWith('0x')) {
    try {
      return { ok: true, value: BigInt(raw) };
    } catch {
      return {
        ok: false,
        code: 'validation_error',
        message: `Prepared EVM ${label} is malformed.`,
      };
    }
  }
  if (!/^\d+$/.test(raw)) {
    return {
      ok: false,
      code: 'validation_error',
      message: `Prepared EVM ${label} is malformed.`,
    };
  }
  try {
    return { ok: true, value: BigInt(raw) };
  } catch {
    return {
      ok: false,
      code: 'validation_error',
      message: `Prepared EVM ${label} is malformed.`,
    };
  }
}

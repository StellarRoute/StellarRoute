import type { AdapterNetworkId } from '@/lib/wallet/adapters';

export type CaipEvmParseResult =
  | { ok: true; namespace: 'eip155'; reference: string; chainIdHex: string }
  | { ok: false; code: string; message: string };

const MAX_EVM_CHAIN_REFERENCE = BigInt(Number.MAX_SAFE_INTEGER);

/** Convert CAIP-2 `eip155:<decimal>` to EIP-1193 hex `0x…`. */
export function caip2EvmToChainIdHex(chainId: string): CaipEvmParseResult {
  const trimmed = chainId.trim();
  if (!trimmed.startsWith('eip155:')) {
    return {
      ok: false,
      code: 'invalid_caip',
      message: 'Expected an eip155 CAIP-2 chain id.',
    };
  }
  const reference = trimmed.slice('eip155:'.length);
  if (!/^\d+$/.test(reference)) {
    return {
      ok: false,
      code: 'invalid_caip',
      message: 'EVM chain reference must be a decimal string.',
    };
  }
  let decimal: bigint;
  try {
    decimal = BigInt(reference);
  } catch {
    return {
      ok: false,
      code: 'invalid_caip',
      message: 'EVM chain reference is not a valid integer.',
    };
  }
  if (decimal < BigInt(0) || decimal > MAX_EVM_CHAIN_REFERENCE) {
    return {
      ok: false,
      code: 'chain_overflow',
      message: 'EVM chain id exceeds supported bounds.',
    };
  }
  return {
    ok: true,
    namespace: 'eip155',
    reference,
    chainIdHex: `0x${decimal.toString(16)}`,
  };
}

export function assertSepoliaCaip(chainId: string): CaipEvmParseResult {
  const parsed = caip2EvmToChainIdHex(chainId);
  if (!parsed.ok) return parsed;
  if (parsed.reference !== '11155111') {
    return {
      ok: false,
      code: 'network_mismatch',
      message: 'Only Sepolia testnet is supported for CCTP EVM execution.',
    };
  }
  return parsed;
}

export function caip2FromChainIdHex(chainIdHex: string): AdapterNetworkId {
  const normalized = chainIdHex.startsWith('0x')
    ? chainIdHex
    : `0x${chainIdHex}`;
  const decimal = Number.parseInt(normalized, 16);
  if (!Number.isFinite(decimal)) {
    return `eip155:${normalized}` as AdapterNetworkId;
  }
  return `eip155:${decimal}` as AdapterNetworkId;
}

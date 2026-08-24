import { getWindowRecord, hasCallable, readPath } from '../detect';

export type SolanaPublicKey = {
  toString(): string;
  toBase58?: () => string;
};

export type SolanaConnectResult = {
  publicKey: SolanaPublicKey;
};

export type SolanaSignMessageResult = {
  signature: Uint8Array | number[];
  publicKey?: SolanaPublicKey;
};

export type SolanaSignTransactionResult = {
  serialize?: () => Uint8Array;
} | Uint8Array;

export type SolanaInjectedWallet = {
  isPhantom?: boolean;
  publicKey?: SolanaPublicKey | null;
  isConnected?: boolean;
  connect: (opts?: { onlyIfTrusted?: boolean }) => Promise<SolanaConnectResult>;
  disconnect?: () => Promise<void>;
  signMessage?: (
    message: Uint8Array,
    display?: string
  ) => Promise<SolanaSignMessageResult | Uint8Array>;
  signTransaction?: (transaction: unknown) => Promise<SolanaSignTransactionResult>;
  signAndSendTransaction?: (
    transaction: unknown,
    options?: { skipPreflight?: boolean; maxRetries?: number }
  ) => Promise<{ signature: string }>;
  /** Some wallets expose cluster via a getter or property. */
  network?: string;
  chain?: string;
  rpcEndpoint?: string;
};

function asWallet(value: unknown): SolanaInjectedWallet | null {
  if (!hasCallable(value, 'connect')) return null;
  return value as SolanaInjectedWallet;
}

/**
 * Resolve an injected Solana wallet.
 * Prefers Phantom (`window.phantom.solana` / `window.solana.isPhantom`).
 */
export function getInjectedSolanaWallet(): SolanaInjectedWallet | null {
  const win = getWindowRecord();
  if (!win) return null;

  const phantom = asWallet(readPath(win, ['phantom', 'solana']));
  if (phantom) return phantom;

  const solana = asWallet(win.solana);
  if (solana) return solana;

  return null;
}

export function publicKeyToAddress(key: SolanaPublicKey): string {
  if (typeof key.toBase58 === 'function') return key.toBase58();
  return key.toString();
}

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i]!);
  }
  if (typeof btoa === 'function') {
    return btoa(binary);
  }
  // Node / vitest fallback
  return Buffer.from(bytes).toString('base64');
}

export function base64ToBytes(value: string): Uint8Array {
  if (typeof atob === 'function') {
    const binary = atob(value);
    const out = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) {
      out[i] = binary.charCodeAt(i);
    }
    return out;
  }
  return new Uint8Array(Buffer.from(value, 'base64'));
}

export function coerceTransactionBytes(
  transaction: string | number[] | Uint8Array,
  encoding: 'base64' | 'bytes' = 'base64'
): Uint8Array {
  if (transaction instanceof Uint8Array) return transaction;
  if (Array.isArray(transaction)) return new Uint8Array(transaction);
  if (encoding === 'base64' || /^[A-Za-z0-9+/=]+$/.test(transaction)) {
    return base64ToBytes(transaction);
  }
  // hex fallback
  const hex = transaction.startsWith('0x') ? transaction.slice(2) : transaction;
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i += 1) {
    out[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

/**
 * Runtime check for a Phantom / web3.js Transaction-like handle.
 * Raw serialized bytes alone are not accepted as production signing inputs.
 */
export function isSolanaWalletTransaction(
  value: unknown
): value is { serialize: (...args: unknown[]) => Uint8Array } {
  return hasCallable(value, 'serialize');
}

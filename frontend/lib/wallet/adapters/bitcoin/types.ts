/** Injected Bitcoin wallet provider shapes (browser extensions). */

export type BitcoinNetworkRaw = 'livenet' | 'testnet' | 'signet' | string;

export type UnisatProvider = {
  requestAccounts: () => Promise<string[]>;
  getAccounts: () => Promise<string[]>;
  getNetwork: () => Promise<BitcoinNetworkRaw>;
  switchNetwork?: (network: 'livenet' | 'testnet') => Promise<void>;
  getPublicKey?: () => Promise<string>;
  signMessage: (
    message: string,
    type?: 'ecdsa' | 'bip322-simple'
  ) => Promise<string>;
  signPsbt: (
    psbtHex: string,
    options?: {
      autoFinalized?: boolean;
      toSignInputs?: Array<{
        index: number;
        address?: string;
        publicKey?: string;
        sighashTypes?: number[];
      }>;
    }
  ) => Promise<string>;
  on?: (event: string, handler: (...args: unknown[]) => void) => void;
  removeListener?: (
    event: string,
    handler: (...args: unknown[]) => void
  ) => void;
};

export type OkxBitcoinProvider = {
  connect: () => Promise<{ address: string; publicKey?: string } | string[]>;
  getAccounts?: () => Promise<string[]>;
  getNetwork?: () => Promise<BitcoinNetworkRaw>;
  switchNetwork?: (network: 'livenet' | 'testnet') => Promise<void>;
  getPublicKey?: () => Promise<string>;
  signMessage: (
    message: string,
    type?: 'ecdsa' | 'bip322-simple'
  ) => Promise<string>;
  signPsbt: (
    psbtHex: string,
    options?: Record<string, unknown>
  ) => Promise<string>;
};

declare global {
  interface Window {
    unisat?: UnisatProvider;
    okxwallet?: {
      bitcoin?: OkxBitcoinProvider;
    };
  }
}

export {};

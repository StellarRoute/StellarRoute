/**
 * Chain-agnostic wallet adapter contracts.
 *
 * Stellar browser wallets remain in `../index.ts` / `../types.ts` and keep
 * powering the existing Freighter/xBull/Albedo/LOBSTR UI.
 *
 * Multi-chain adapters (EVM, Solana, Bitcoin, TRON) implement
 * `ChainWalletAdapter` and register via `registry.ts`.
 *
 * Security: adapters MUST only talk to injected browser wallets.
 * Never accept, store, or transmit seed phrases or private keys.
 */

/** Stable chain families shared across parallel wallet workstreams. */
export type ChainFamily = 'stellar' | 'bitcoin' | 'tron' | 'evm' | 'solana';

/**
 * Canonical network ids per family.
 * EVM uses CAIP-2 `eip155:<chainId>`; Solana uses `solana:<cluster>`.
 */
export type AdapterNetworkId =
  | 'bitcoin:mainnet'
  | 'bitcoin:testnet'
  | 'bitcoin:signet'
  | 'tron:mainnet'
  | 'tron:nile'
  | 'tron:shasta'
  | 'stellar:public'
  | 'stellar:testnet'
  | 'stellar:futurenet'
  | 'eip155:1'
  | 'eip155:11155111'
  | 'eip155:8453'
  | 'eip155:84532'
  | 'eip155:42161'
  | 'eip155:421614'
  | 'solana:mainnet'
  | 'solana:devnet'
  | 'solana:testnet'
  | (string & {});

export type AdapterCapability =
  | 'connect'
  | 'disconnect'
  | 'view_address'
  | 'view_network'
  | 'sign_message'
  | 'sign_transaction'
  | 'send_transaction'
  | 'switch_network';

export type AdapterCapabilityStatus = {
  capability: AdapterCapability;
  allowed: boolean;
  reason?: string;
  resolution?: string;
};

export type AdapterCapabilities = {
  checkedAt: number;
  statuses: AdapterCapabilityStatus[];
};

export type ChainWalletAccount = {
  address: string;
  /** Optional hex/base58 public key when the wallet exposes it. */
  publicKey?: string;
};

export type ChainWalletSession = {
  adapterId: string;
  chainFamily: ChainFamily;
  account: ChainWalletAccount;
  network: AdapterNetworkId;
  isConnected: boolean;
};

export type ChainNetworkInfo = {
  network: AdapterNetworkId;
  /** Raw wallet-reported label (e.g. `0x1`, `mainnet-beta`). */
  raw?: string;
  /** True when wallet network matches the expected app network. */
  matchesExpected: boolean;
  expected?: AdapterNetworkId;
};

/**
 * Opaque signing payloads — chain-specific shapes live behind `kind`.
 * Keys never leave the wallet extension.
 */
export type SignMessageRequest = {
  kind: 'message';
  message: string;
  /** Bitcoin: `ecdsa` | `bip322-simple`. TRON/EVM/Solana: ignored or mapped. */
  encoding?: 'utf8' | 'hex';
  bitcoinSignType?: 'ecdsa' | 'bip322-simple';
};

/** Minimal EIP-1193 transaction fields used for sign/send. */
export type EvmTransactionRequest = {
  from?: string;
  to?: string;
  value?: string;
  data?: string;
  gas?: string;
  gasPrice?: string;
  maxFeePerGas?: string;
  maxPriorityFeePerGas?: string;
  nonce?: string;
  chainId?: string;
};

/** Phantom / Wallet-Standard compatible transaction handle. */
export type SolanaWalletTransaction = {
  serialize: (...args: unknown[]) => Uint8Array;
};

export type SignTransactionRequest =
  | {
      kind: 'bitcoin_psbt';
      /** PSBT as hex (UniSat/OKX) or base64 — adapters normalize. */
      psbt: string;
      format?: 'hex' | 'base64';
      options?: {
        autoFinalized?: boolean;
        toSignInputs?: Array<{
          index: number;
          address?: string;
          publicKey?: string;
          sighashTypes?: number[];
        }>;
      };
    }
  | {
      kind: 'tron_transaction';
      /** Unsigned TRON transaction object from TronWeb / backend. */
      transaction: Record<string, unknown>;
    }
  | {
      kind: 'evm_transaction';
      transaction: EvmTransactionRequest;
    }
  | {
      kind: 'solana_transaction';
      /**
       * Wallet-compatible Transaction / VersionedTransaction handle.
       * Must expose `serialize()` (runtime-validated). Raw base64 / byte
       * arrays alone are rejected with `unsupported_capability` until a
       * real @solana/web3.js (or equivalent) decode path exists.
       */
      transaction: SolanaWalletTransaction | string | number[] | Uint8Array;
      encoding?: 'base64' | 'bytes';
    }
  | {
      kind: 'stellar_xdr';
      xdr: string;
      networkPassphrase?: string;
      publicKey?: string;
    };

export type SignedMessageResult = {
  signature: string;
  address: string;
  publicKey?: string;
};

export type SignedTransactionResult =
  | {
      kind: 'bitcoin_psbt';
      /** Signed PSBT hex/base64 — never a raw private key. */
      psbt: string;
      format: 'hex' | 'base64';
    }
  | {
      kind: 'tron_transaction';
      /** Signed TRON transaction object ready for broadcast by the wallet or backend. */
      transaction: Record<string, unknown>;
    }
  | {
      kind: 'evm_transaction';
      /** Signed raw tx hex when the wallet supports eth_signTransaction. */
      signedTransaction?: string;
      /** Present when the wallet broadcast via eth_sendTransaction. */
      hash?: string;
    }
  | {
      kind: 'solana_transaction';
      /** Base64 signed transaction bytes when only signing. */
      signedTransaction?: string;
      /** Present when the wallet broadcast via signAndSendTransaction. */
      signature?: string;
    }
  | {
      kind: 'stellar_xdr';
      signedXdr: string;
    };

export type SendTransactionRequest =
  | {
      kind: 'evm_transaction';
      transaction: EvmTransactionRequest;
    }
  | {
      kind: 'solana_transaction';
      transaction: SolanaWalletTransaction | string | number[] | Uint8Array;
      encoding?: 'base64' | 'bytes';
      options?: { skipPreflight?: boolean; maxRetries?: number };
    };

export type SendTransactionResult =
  | {
      kind: 'evm_transaction';
      hash: string;
    }
  | {
      kind: 'solana_transaction';
      signature: string;
    };

/**
 * Cross-chain swap execution status relative to backend routes.
 * Do not claim executable swaps when routes are absent.
 */
export type ExecutionSupportKind =
  | 'supported'
  | 'signing_only'
  | 'unsupported'
  | 'degraded';

export type ExecutionSupport = {
  kind: ExecutionSupportKind;
  /** Machine-readable reason code for UI / telemetry. */
  code:
    | 'stellar_native'
    | 'chain_signing_available'
    | 'no_backend_route'
    | 'network_mismatch'
    | 'wallet_capability_missing'
    | 'not_connected';
  message: string;
  /** Optional remediation copy for the UI. */
  resolution?: string;
};

export type AvailableChainWallet = {
  id: string;
  label: string;
  chainFamily: ChainFamily;
  installed: boolean;
  installUrl?: string;
};

export type ChainWalletAdapter = {
  readonly id: string;
  readonly label: string;
  readonly chainFamily: ChainFamily;
  readonly installUrl?: string;

  /** Safe presence check — must not throw; timeouts allowed. */
  detectInstalled(): Promise<boolean>;

  connect(expectedNetwork?: AdapterNetworkId): Promise<ChainWalletSession>;

  /**
   * Best-effort disconnect. Many extensions only drop the dapp session;
   * never attempts to wipe wallet keys.
   */
  disconnect(): Promise<void>;

  /** Passive session read when possible; may return null if locked. */
  getSession(): Promise<ChainWalletSession | null>;

  getNetwork(expectedNetwork?: AdapterNetworkId): Promise<ChainNetworkInfo>;

  /**
   * Request a network switch when the wallet supports it.
   * Returns the post-switch network info, or throws if unsupported.
   */
  switchNetwork?(network: AdapterNetworkId): Promise<ChainNetworkInfo>;

  signMessage(request: SignMessageRequest): Promise<SignedMessageResult>;

  signTransaction(
    request: SignTransactionRequest
  ): Promise<SignedTransactionResult>;

  /**
   * Broadcast via the wallet when supported (eth_sendTransaction /
   * signAndSendTransaction). Optional — not all chains expose send.
   */
  sendTransaction?(
    request: SendTransactionRequest
  ): Promise<SendTransactionResult>;

  checkCapabilities(
    expectedNetwork?: AdapterNetworkId
  ): Promise<AdapterCapabilities>;

  /**
   * Whether this adapter can participate in an app swap for `routeHint`.
   * Defaults to signing-only / unsupported when no backend route exists.
   */
  getExecutionSupport(routeHint?: {
    sourceChain?: ChainFamily;
    destinationChain?: ChainFamily;
  }): ExecutionSupport;
};

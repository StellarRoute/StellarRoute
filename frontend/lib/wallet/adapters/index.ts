/**
 * Multi-chain wallet adapters.
 *
 * Stellar Freighter/xBull/Albedo/LOBSTR continue to live in `../index.ts`
 * for the existing swap UI. Thin Stellar wrappers plus EVM, Solana,
 * Bitcoin, and TRON adapters register here for `useChainWallet`.
 */

export type {
  AdapterCapabilities,
  AdapterCapability,
  AdapterCapabilityStatus,
  AdapterNetworkId,
  AvailableChainWallet,
  ChainFamily,
  ChainNetworkInfo,
  ChainWalletAccount,
  ChainWalletAdapter,
  ChainWalletSession,
  EvmTransactionRequest,
  ExecutionSupport,
  ExecutionSupportKind,
  SendTransactionRequest,
  SendTransactionResult,
  SignMessageRequest,
  SignTransactionRequest,
  SignedMessageResult,
  SignedTransactionResult,
  SolanaWalletTransaction,
} from './types';

export {
  WalletAdapterError,
  normalizeProviderError,
  isUserRejection,
  isRpcMethodNotFound,
} from './errors';
export type { WalletAdapterErrorCode } from './errors';

export {
  getWindowRecord,
  withTimeout,
  readPath,
  hasCallable,
} from './detect';

export {
  resolveExecutionSupport,
  chainSigningSupport,
  hasBackendRoute,
  routeKey,
} from './execution-support';

export {
  registerAdapter,
  unregisterAdapter,
  getAdapter,
  listAdapters,
  listAvailableChainWallets,
  ensureDefaultAdapters,
  clearAdaptersForTests,
} from './registry';

export {
  createEmptyChainWalletState,
  refreshAvailableWallets,
  connectChainWallet,
  disconnectChainWallet,
  signWithChainWallet,
  signMessageWithChainWallet,
  sendWithChainWallet,
  getChainExecutionSupport,
} from './session';
export type { ChainWalletState } from './session';

export {
  createInjectedEvmAdapter,
  createWalletConnectEvmAdapter,
  EVM_NETWORKS,
  defaultEvmAppNetwork,
  isWalletConnectConfigured,
} from './evm';
export {
  createInjectedSolanaAdapter,
  SOLANA_NETWORKS,
  defaultSolanaAppNetwork,
} from './solana';
export { createStellarWalletAdapter, createAllStellarAdapters } from './stellar/legacy';

export {
  createUnisatAdapter,
  createOkxBitcoinAdapter,
  normalizeBitcoinNetwork,
  bitcoinNetworkToUnisat,
  bitcoinNetworksMatch,
} from './bitcoin';

export {
  createTronLinkAdapter,
  normalizeTronNetwork,
  tronNetworksMatch,
} from './tron';

export {
  createStubBridgeProvider,
  type BridgeExecutionProvider,
  type BridgeQuoteRequest,
  type BridgeRouteHint,
} from './bridge';

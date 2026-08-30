export interface WalletPickerOption {
  id: string;
  label: string;
  installed: boolean;
}

/** Lifted wallet session passed from CrossChainSwapDeck into visual chips and saga. */
export interface WalletChipBinding {
  chainLabel: string;
  chainShortLabel: string;
  testId: string;
  address: string | null;
  isConnecting: boolean;
  isConnected: boolean;
  networkMismatch: boolean;
  unsupported?: boolean;
  availableWallets: WalletPickerOption[];
  onConnect: (walletId: string) => Promise<void>;
  onDisconnect?: () => Promise<void>;
  /** When true the chip is display-only (recipient address, not a signer). */
  readOnly?: boolean;
}

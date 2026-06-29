const MOCK_ADDRESS =
  'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

export const isAllowed = async () => ({ isAllowed: true });
export const requestAccess = async () => ({ address: MOCK_ADDRESS });
export const getAddress = async () => ({ address: MOCK_ADDRESS });
export const getNetworkDetails = async () => ({
  network: 'testnet',
  networkUrl: 'https://horizon-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
});
export const signTransaction = async () => ({
  signedTxXdr: '',
  signerAddress: MOCK_ADDRESS,
});
export const isConnected = async () => ({ isConnected: true });
export const getNetwork = async () => ({
  network: 'testnet',
  networkPassphrase: 'Test SDF Network ; September 2015',
});
export const setAllowed = async () => ({ isAllowed: true });
export const WatchWalletChanges = () => {};

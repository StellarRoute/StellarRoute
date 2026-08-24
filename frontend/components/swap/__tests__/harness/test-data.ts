export interface MockAsset {
  symbol: string;
  name: string;
  decimals: number;
  balance: string;
  icon?: string;
}

export const MOCK_ASSETS: MockAsset[] = [
  { 
    symbol: 'XLM', 
    name: 'Stellar Lumens', 
    decimals: 7, 
    balance: '1000.00',
    icon: '/icons/xlm.png' 
  },
  { 
    symbol: 'USDC', 
    name: 'USD Coin', 
    decimals: 6, 
    balance: '50.25',
    icon: '/icons/usdc.png'
  }
];

export const MOCK_QUOTE = {
  fromAmount: '100',
  toAmount: '11.85',
  rate: '0.1185',
  fee: '0.00001'
};
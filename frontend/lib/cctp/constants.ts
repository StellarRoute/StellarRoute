import type { ChainAsset } from './types';
import {
  CCTP_PROVIDER_ID,
  CCTP_TESTNET_CORRIDOR_ID,
  SEPOLIA_CHAIN_ID,
  STELLAR_TESTNET_CHAIN_ID,
} from './types';

/** Documented Sepolia CCTP contracts (validation only — payloads are server-built). */
export const SEPOLIA_CCTP_CONTRACTS = {
  tokenMessenger: '0x8fe6b999dc680ccfdd5bf7eb0974218be2542daa',
  messageTransmitter: '0xe737e5cebeeba77efe34d4aa090756590b1ce275',
  usdc: '0x1c7d4b196cb0c7b01d743fbc6116a902379c7238',
} as const;

export const STELLAR_TESTNET_USDC: ChainAsset = {
  chain_id: STELLAR_TESTNET_CHAIN_ID,
  asset: 'erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA',
  canonical:
    'stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA',
  symbol: 'USDC',
};

export const SEPOLIA_USDC: ChainAsset = {
  chain_id: SEPOLIA_CHAIN_ID,
  asset: 'erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238',
  canonical:
    'eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238',
  symbol: 'USDC',
};

export const CCTP_CORRIDOR_DEFAULTS = {
  corridorId: CCTP_TESTNET_CORRIDOR_ID,
  provider: CCTP_PROVIDER_ID,
} as const;

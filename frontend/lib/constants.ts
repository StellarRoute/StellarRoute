export const API_PROXY_ENABLED =
  process.env.NEXT_PUBLIC_API_PROXY === 'true';

export const API_BASE_URL = API_PROXY_ENABLED
  ? '/api/v1'
  : process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080/api/v1';

export const APP_NAME = 'StellarRoute';
export const APP_DESCRIPTION =
  'Best-price routing across Stellar DEX and Soroban AMM pools';

export const ROUTES = {
  HOME: '/',
  SWAP: '/swap',
} as const;

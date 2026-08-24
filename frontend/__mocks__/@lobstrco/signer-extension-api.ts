import { vi } from 'vitest';

export const isConnected = vi.fn().mockResolvedValue(false);
export const getPublicKey = vi.fn().mockResolvedValue('');
export const signTransaction = vi.fn().mockResolvedValue('');
export const signMessage = vi.fn().mockResolvedValue(null);

export default {
  isConnected,
  getPublicKey,
  signTransaction,
  signMessage,
};

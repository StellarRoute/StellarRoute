import type { ExecutionTimelineStep } from './types';

export const STELLAR_NATIVE_TIMELINE_IDLE: ExecutionTimelineStep[] = [
  {
    id: 'stellar_swap',
    label: 'Stellar swap',
    description: 'Sign and submit via Stellar wallet when you review.',
    status: 'pending',
  },
  {
    id: 'burn',
    label: 'Burn',
    description: 'Not used for same-chain Stellar swaps.',
    status: 'unavailable',
  },
  {
    id: 'attest',
    label: 'Attest',
    description: 'Not used for same-chain Stellar swaps.',
    status: 'unavailable',
  },
  {
    id: 'mint',
    label: 'Mint',
    description: 'Not used for same-chain Stellar swaps.',
    status: 'unavailable',
  },
];

export const PREVIEW_TIMELINE_UNAVAILABLE: ExecutionTimelineStep[] = [
  {
    id: 'stellar_swap',
    label: 'Stellar swap',
    description: 'Available when this corridor is executable.',
    status: 'unavailable',
  },
  {
    id: 'burn',
    label: 'Burn',
    description: 'Source-chain burn — protocol preview only.',
    status: 'unavailable',
  },
  {
    id: 'attest',
    label: 'Attest',
    description: 'Circle attestation — timing varies by corridor.',
    status: 'unavailable',
  },
  {
    id: 'mint',
    label: 'Mint',
    description: 'Destination mint after attestation.',
    status: 'unavailable',
  },
];

export function buildExecutionTimelineSteps(
  executable: boolean,
  isStellarNativeExecutable: boolean
): ExecutionTimelineStep[] {
  if (!executable) {
    return PREVIEW_TIMELINE_UNAVAILABLE;
  }
  if (isStellarNativeExecutable) {
    return STELLAR_NATIVE_TIMELINE_IDLE;
  }
  return PREVIEW_TIMELINE_UNAVAILABLE;
}

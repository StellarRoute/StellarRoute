'use client';

import { useMemo } from 'react';

import { useWallet } from '@/components/providers/wallet-provider';
import { createStellarRouteClient } from '@/lib/api/client';
import { getApiBaseUrl } from '@/lib/network-endpoints';

export function useStellarRouteClient() {
  const { network } = useWallet();
  return useMemo(
    () => createStellarRouteClient(getApiBaseUrl(network)),
    [network],
  );
}

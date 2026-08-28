'use client';

import { useCallback, useMemo } from 'react';
import { useWallet } from '@/components/providers/wallet-provider';
import { useChainWallet } from '@/hooks/useChainWallet';
import type { ChainDisplayId } from '@/lib/cross-chain/types';
import { CHAIN_DEFINITIONS } from '@/lib/cross-chain/corridors';
import { resolveCctpDirection } from '@/lib/cctp/corridor-bridge';
import type { WalletChipBinding } from '@/lib/cross-chain/wallet-chip-types';
import type { CctpWalletRoles } from '@/hooks/useCctpSaga';
import { StrKey } from '@stellar/stellar-base';
import type { SupportedWallet } from '@/lib/wallet/types';

const SEPOLIA_NETWORK = 'eip155:11155111' as const;
const STELLAR_TESTNET = 'stellar:testnet' as const;

export interface UseCrossChainWalletRolesInput {
  sourceChainId: ChainDisplayId;
  destChainId: ChainDisplayId;
  recipientOverride?: string;
  useRecipientOverride?: boolean;
}

export function useCrossChainWalletRoles(input: UseCrossChainWalletRolesInput) {
  const direction = resolveCctpDirection(input.sourceChainId, input.destChainId);
  const sourceChain = CHAIN_DEFINITIONS[input.sourceChainId];
  const destChain = CHAIN_DEFINITIONS[input.destChainId];

  const stellarSource = useWallet();
  const evmSource = useChainWallet({
    chainFamily: 'evm',
    expectedNetwork: SEPOLIA_NETWORK,
  });
  const evmDestination = useChainWallet({
    chainFamily: 'evm',
    expectedNetwork: SEPOLIA_NETWORK,
  });
  const stellarMintSubmitter = useChainWallet({
    chainFamily: 'stellar',
    expectedNetwork: STELLAR_TESTNET,
  });

  const destRecipientAddress = useMemo(() => {
    if (input.useRecipientOverride && input.recipientOverride?.trim()) {
      return input.recipientOverride.trim();
    }
    if (direction === 'stellar_to_evm') {
      return evmDestination.address ?? '';
    }
    if (direction === 'evm_to_stellar') {
      return stellarMintSubmitter.address ?? stellarSource.address ?? '';
    }
    return '';
  }, [
    direction,
    evmDestination.address,
    input.recipientOverride,
    input.useRecipientOverride,
    stellarMintSubmitter.address,
    stellarSource.address,
  ]);

  const isMuxedRecipient = StrKey.isValidMed25519PublicKey(destRecipientAddress);
  const showMintSubmitterChip =
    direction === 'evm_to_stellar' || isMuxedRecipient;

  const stellarSourceBinding = useMemo(
    (): WalletChipBinding => ({
      chainLabel: sourceChain.label,
      chainShortLabel: sourceChain.shortLabel,
      testId: 'wallet-chip-stellar',
      address: stellarSource.address,
      isConnecting: stellarSource.isLoading,
      isConnected: stellarSource.isConnected,
      networkMismatch: stellarSource.networkMismatch,
      availableWallets: [
        { id: 'freighter', label: 'Freighter', installed: true },
        { id: 'xbull', label: 'xBull', installed: true },
        { id: 'albedo', label: 'Albedo', installed: true },
        { id: 'lobstr', label: 'LOBSTR', installed: true },
      ],
      onConnect: async (walletId) => {
        await stellarSource.connect(walletId as SupportedWallet);
      },
      onDisconnect: async () => {
        await stellarSource.disconnect();
      },
    }),
    [sourceChain.label, sourceChain.shortLabel, stellarSource],
  );

  const buildEvmBinding = useCallback(
    (
      hook: ReturnType<typeof useChainWallet>,
      chainId: ChainDisplayId,
      testId: string,
    ): WalletChipBinding => {
      const chain = CHAIN_DEFINITIONS[chainId];
      return {
        chainLabel: chain.label,
        chainShortLabel: chain.shortLabel,
        testId,
        address: hook.address,
        isConnecting: hook.isLoading,
        isConnected: hook.isConnected,
        networkMismatch: hook.networkMismatch,
        availableWallets: hook.availableWallets.map((w) => ({
          id: w.id,
          label: w.label,
          installed: w.installed,
        })),
        onConnect: async (walletId) => {
          await hook.connect(walletId);
        },
        onDisconnect: async () => {
          await hook.disconnect();
        },
      };
    },
    [],
  );

  const buildStellarMintBinding = useCallback((): WalletChipBinding => {
    const address =
      stellarMintSubmitter.address ?? stellarSource.address ?? null;
    const connected =
      stellarMintSubmitter.isConnected || stellarSource.isConnected;
    return {
      chainLabel: 'Stellar mint submitter',
      chainShortLabel: 'Fee payer (G)',
      testId: 'wallet-chip-stellar-mint-submitter',
      address,
      isConnecting:
        stellarMintSubmitter.isLoading || stellarSource.isLoading,
      isConnected: connected,
      networkMismatch:
        stellarMintSubmitter.networkMismatch || stellarSource.networkMismatch,
      availableWallets: stellarMintSubmitter.availableWallets.length
        ? stellarMintSubmitter.availableWallets.map((w) => ({
            id: w.id,
            label: w.label,
            installed: w.installed,
          }))
        : [
            { id: 'freighter', label: 'Freighter', installed: true },
            { id: 'xbull', label: 'xBull', installed: true },
          ],
      onConnect: async (walletId) => {
        if (stellarMintSubmitter.availableWallets.length) {
          await stellarMintSubmitter.connect(walletId);
        } else {
          await stellarSource.connect(walletId as SupportedWallet);
        }
      },
      onDisconnect: async () => {
        if (stellarMintSubmitter.isConnected) {
          await stellarMintSubmitter.disconnect();
        }
      },
    };
  }, [stellarMintSubmitter, stellarSource]);

  const sourceChipBinding = useMemo((): WalletChipBinding | null => {
    if (sourceChain.chainFamily === 'stellar') return stellarSourceBinding;
    if (sourceChain.chainFamily === 'evm') {
      return buildEvmBinding(evmSource, input.sourceChainId, `wallet-chip-${input.sourceChainId}`);
    }
    return null;
  }, [
    buildEvmBinding,
    evmSource,
    input.sourceChainId,
    sourceChain.chainFamily,
    stellarSourceBinding,
  ]);

  const destChipBinding = useMemo((): WalletChipBinding | null => {
    if (destChain.chainFamily === 'evm') {
      return buildEvmBinding(
        evmDestination,
        input.destChainId,
        `wallet-chip-${input.destChainId}`,
      );
    }
    if (destChain.chainFamily === 'stellar' && direction === 'evm_to_stellar') {
      return {
        ...stellarSourceBinding,
        chainLabel: 'Stellar recipient',
        chainShortLabel: isMuxedRecipient ? 'Muxed (M)' : 'Account (G)',
        testId: 'wallet-chip-stellar-recipient',
        address: destRecipientAddress || stellarSource.address,
        readOnly: Boolean(
          input.useRecipientOverride && input.recipientOverride?.trim(),
        ),
      };
    }
    return null;
  }, [
    buildEvmBinding,
    destChain.chainFamily,
    destRecipientAddress,
    direction,
    evmDestination,
    input.destChainId,
    input.recipientOverride,
    input.useRecipientOverride,
    isMuxedRecipient,
    stellarSource.address,
    stellarSourceBinding,
  ]);

  const mintSubmitterChipBinding = useMemo(
    (): WalletChipBinding | null =>
      showMintSubmitterChip ? buildStellarMintBinding() : null,
    [buildStellarMintBinding, showMintSubmitterChip],
  );

  const sagaWallets = useMemo((): CctpWalletRoles => {
    const mintSubmitterAddress =
      stellarMintSubmitter.address ?? stellarSource.address ?? undefined;
    return {
      sourceStellarAdapterId:
        sourceChain.chainFamily === 'stellar'
          ? stellarSource.walletId ?? undefined
          : undefined,
      sourceEvmAdapterId:
        sourceChain.chainFamily === 'evm'
          ? evmSource.adapterId ?? undefined
          : undefined,
      evmDestinationAdapterId:
        destChain.chainFamily === 'evm'
          ? evmDestination.adapterId ?? undefined
          : undefined,
      mintSubmitterStellarAdapterId:
        stellarMintSubmitter.adapterId ?? stellarSource.walletId ?? undefined,
      sourceAddress:
        sourceChain.chainFamily === 'stellar'
          ? stellarSource.address ?? undefined
          : evmSource.address ?? undefined,
      recipient: destRecipientAddress,
      mintSubmitter:
        direction === 'evm_to_stellar' ? mintSubmitterAddress : undefined,
    };
  }, [
    destChain.chainFamily,
    destRecipientAddress,
    direction,
    evmDestination.adapterId,
    evmSource.adapterId,
    evmSource.address,
    sourceChain.chainFamily,
    stellarMintSubmitter.adapterId,
    stellarMintSubmitter.address,
    stellarSource.address,
    stellarSource.walletId,
  ]);

  return {
    direction,
    destRecipientAddress,
    isMuxedRecipient,
    showMintSubmitterChip,
    sourceChipBinding,
    destChipBinding,
    mintSubmitterChipBinding,
    sagaWallets,
  };
}

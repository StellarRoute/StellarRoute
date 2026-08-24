'use client';

import { ArrowDown } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ChainDisplayId } from '@/lib/cross-chain/types';
import { CHAIN_DEFINITIONS } from '@/lib/cross-chain/corridors';
import type { WalletChipBinding } from '@/lib/cross-chain/wallet-chip-types';
import { ChainWalletChip } from './ChainWalletChip';
import type { CrossChainWalletStoryState } from './crossChainStoryPresentation';

interface PairedChainSelectorsProps {
  sourceChainId: ChainDisplayId;
  destChainId: ChainDisplayId;
  onSourceChange: (id: ChainDisplayId) => void;
  onDestChange: (id: ChainDisplayId) => void;
  sourceWalletState?: CrossChainWalletStoryState;
  destWalletState?: CrossChainWalletStoryState;
  sourceWalletBinding?: WalletChipBinding | null;
  destWalletBinding?: WalletChipBinding | null;
  mintSubmitterBinding?: WalletChipBinding | null;
  inputsLocked?: boolean;
  destWalletHint?: string | null;
}

const SOURCE_CHAINS: ChainDisplayId[] = [
  'stellar',
  'ethereum-sepolia',
  'solana',
  'bitcoin',
  'tron',
];

/** Destinations stay Stellar-centered; extras stay available for custom pairs. */
const DEST_CHAINS: ChainDisplayId[] = [
  'stellar',
  'ethereum-sepolia',
  'solana',
  'bitcoin',
  'tron',
];

export function PairedChainSelectors({
  sourceChainId,
  destChainId,
  onSourceChange,
  onDestChange,
  sourceWalletState,
  destWalletState,
  sourceWalletBinding,
  destWalletBinding,
  mintSubmitterBinding,
  inputsLocked = false,
  destWalletHint,
}: PairedChainSelectorsProps) {
  return (
    <section
      aria-label="Source and destination chains"
      className="overflow-hidden rounded-[1.5rem] border border-border/40 bg-card/40"
      data-testid="paired-chain-selectors"
    >
      {inputsLocked && (
        <p
          className="border-b border-border/40 bg-muted/20 px-4 py-2.5 text-xs text-muted-foreground sm:px-5"
          role="status"
          data-testid="cctp-inputs-locked-banner"
        >
          Transfer in progress — chains and amount are locked. Wallets stay
          connectable for signing.
        </p>
      )}

      <div className="space-y-0 p-4 sm:p-5">
        <ChainLeg
          role="source"
          chainId={sourceChainId}
          chains={SOURCE_CHAINS}
          onChange={onSourceChange}
          walletStoryState={sourceWalletState}
          walletBinding={sourceWalletBinding}
          inputsLocked={inputsLocked}
        />

        <div className="relative flex items-center justify-center py-3" aria-hidden>
          <span className="absolute inset-x-0 top-1/2 h-px bg-border/50" />
          <span className="relative z-[1] flex size-9 items-center justify-center rounded-full border border-border/60 bg-background text-primary shadow-sm">
            <ArrowDown className="size-4" />
          </span>
        </div>

        <ChainLeg
          role="destination"
          chainId={destChainId}
          chains={DEST_CHAINS}
          onChange={onDestChange}
          walletStoryState={destWalletState}
          walletBinding={destWalletBinding}
          inputsLocked={inputsLocked}
          walletHint={destWalletHint}
        />
      </div>

      {mintSubmitterBinding && (
        <div
          className="space-y-2 border-t border-border/40 bg-muted/10 px-4 py-3 sm:px-5"
          data-testid="stellar-mint-submitter-control"
        >
          <p className="text-xs font-medium text-foreground">
            Stellar wallet for fees
          </p>
          <p className="text-xs text-muted-foreground">
            Connect a Stellar wallet to pay network fees when receiving USDC.
          </p>
          <ChainWalletChip binding={mintSubmitterBinding} />
        </div>
      )}
    </section>
  );
}

function ChainLeg({
  role,
  chainId,
  chains,
  onChange,
  walletStoryState,
  walletBinding,
  inputsLocked,
  walletHint,
}: {
  role: 'source' | 'destination';
  chainId: ChainDisplayId;
  chains: ChainDisplayId[];
  onChange: (id: ChainDisplayId) => void;
  walletStoryState?: CrossChainWalletStoryState;
  walletBinding?: WalletChipBinding | null;
  inputsLocked?: boolean;
  walletHint?: string | null;
}) {
  const chain = CHAIN_DEFINITIONS[chainId];
  const legLabel = role === 'source' ? 'From' : 'To';
  const isConnected =
    walletStoryState === 'connected' ||
    (walletStoryState === undefined && Boolean(walletBinding?.isConnected));
  const showInlineChip =
    Boolean(walletBinding) &&
    (isConnected ||
      walletStoryState === 'connecting' ||
      walletStoryState === 'mismatch' ||
      walletStoryState === 'unsupported' ||
      Boolean(walletBinding?.readOnly));

  return (
    <div className="space-y-3" data-testid={`chain-leg-${role}`}>
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
            {legLabel}
          </p>
          <p className="truncate font-display text-xl font-semibold tracking-tight text-foreground sm:text-2xl">
            {chain.shortLabel}
          </p>
        </div>
        {showInlineChip && (
          <ChainWalletChip
            binding={walletBinding}
            storyState={walletStoryState}
            className="shrink-0"
          />
        )}
      </div>

      {walletBinding && !showInlineChip && (
        <ChainWalletChip
          binding={walletBinding}
          storyState={walletStoryState}
        />
      )}

      {role === 'destination' && walletHint && (
        <p
          className="text-xs leading-relaxed text-muted-foreground"
          role="status"
          data-testid="dest-wallet-setup-hint"
        >
          {walletHint}
        </p>
      )}

      <ChainSelector
        value={chainId}
        onChange={onChange}
        chains={chains}
        label={`${role === 'source' ? 'Source' : 'Destination'} chain`}
        name={`cross-chain-${role}`}
        role={role}
        disabled={inputsLocked}
      />
    </div>
  );
}

function ChainSelector({
  value,
  onChange,
  chains,
  label,
  name,
  role,
  disabled,
}: {
  value: ChainDisplayId;
  onChange: (id: ChainDisplayId) => void;
  chains: ChainDisplayId[];
  label: string;
  name: string;
  role: 'source' | 'destination';
  disabled?: boolean;
}) {
  return (
    <fieldset className="space-y-0" disabled={disabled}>
      <legend className="sr-only">{label}</legend>
      <div
        role="radiogroup"
        aria-label={label}
        className="flex flex-wrap gap-1.5"
      >
        {chains.map((id) => {
          const chain = CHAIN_DEFINITIONS[id];
          const selected = value === id;
          return (
            <label
              key={id}
              className={cn(
                'relative inline-flex min-h-11 cursor-pointer items-center rounded-full border px-3.5 py-2 transition-colors',
                'has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring',
                selected
                  ? 'border-primary/55 bg-primary/12 text-foreground'
                  : 'border-border/40 bg-background/40 text-muted-foreground hover:border-border hover:text-foreground',
                disabled && 'pointer-events-none opacity-60',
              )}
            >
              <input
                type="radio"
                name={name}
                value={id}
                checked={selected}
                disabled={disabled}
                onChange={() => onChange(id)}
                className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                data-testid={`chain-option-${role}-${id}`}
              />
              <span className="relative z-[1] text-xs font-semibold pointer-events-none">
                {chain.shortLabel}
              </span>
            </label>
          );
        })}
      </div>
    </fieldset>
  );
}

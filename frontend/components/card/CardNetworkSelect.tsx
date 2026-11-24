'use client';

import { Label } from '@/components/ui/label';
import { SEPOLIA_CCTP_CONTRACTS } from '@/lib/cctp/constants';
import type { AdapterNetworkId } from '@/lib/wallet/adapters';
import { cn } from '@/lib/utils';

/**
 * A Circle CCTP source network a cardholder can fund the card from.
 *
 * `circleDomain` is the CCTP v2 source domain the API needs to initiate a
 * burn. Only domains the backend actually wires up are `executable`; the
 * others are selectable so the wallet can be prepared on the right chain,
 * but the UI never implies the bridge will run.
 */
export interface CardCctpSourceNetwork {
  /** Stable selector id, e.g. `ethereum-sepolia`. */
  id: string;
  label: string;
  /** CAIP-2 id used by the wallet adapters. */
  network: AdapterNetworkId;
  chainId: number;
  /** Circle CCTP source domain handed to the burn request. */
  circleDomain: number;
  /** Source USDC contract, or null when this build has no verified address. */
  usdcAddress: string | null;
  /** True when StellarRoute can execute a CCTP burn from this chain today. */
  executable: boolean;
}

/**
 * Ethereum Sepolia is the only corridor the API enables today
 * (`SEPOLIA_DOMAIN` / `STELLAR_TESTNET_DOMAIN` in `crates/api/src/cctp/config.rs`).
 * Base Sepolia and Arbitrum Sepolia carry Circle's documented CCTP testnet
 * domain ids so the selection is unambiguous once those corridors open.
 */
export const CARD_CCTP_SOURCE_NETWORKS: readonly CardCctpSourceNetwork[] = [
  {
    id: 'ethereum-sepolia',
    label: 'Ethereum Sepolia',
    network: 'eip155:11155111',
    chainId: 11155111,
    circleDomain: 0,
    usdcAddress: SEPOLIA_CCTP_CONTRACTS.usdc,
    executable: true,
  },
  {
    id: 'base-sepolia',
    label: 'Base Sepolia',
    network: 'eip155:84532',
    chainId: 84532,
    circleDomain: 16,
    usdcAddress: null,
    executable: false,
  },
  {
    id: 'arbitrum-sepolia',
    label: 'Arbitrum Sepolia',
    network: 'eip155:421614',
    chainId: 421614,
    circleDomain: 18,
    usdcAddress: null,
    executable: false,
  },
] as const;

export const DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID = 'ethereum-sepolia';

export function findCardCctpSourceNetwork(
  networks: readonly CardCctpSourceNetwork[],
  id: string
): CardCctpSourceNetwork | null {
  return networks.find((network) => network.id === id) ?? null;
}

/** Resolve a wallet-reported CAIP-2 id back to a listed source network. */
export function cardCctpSourceNetworkForCaip2(
  networks: readonly CardCctpSourceNetwork[],
  network: AdapterNetworkId | null | undefined
): CardCctpSourceNetwork | null {
  if (!network) return null;
  return networks.find((candidate) => candidate.network === network) ?? null;
}

export interface CardNetworkSelectProps {
  /** Currently active source network id. */
  value: string;
  /** Fired when the cardholder picks a different network. */
  onNetworkChange: (network: CardCctpSourceNetwork) => void;
  /** Override the catalogue (tests, future per-account allowlists). */
  networks?: readonly CardCctpSourceNetwork[];
  /** Lock the control while a wallet network switch is in flight. */
  switching?: boolean;
  disabled?: boolean;
  className?: string;
}

export function CardNetworkSelect({
  value,
  onNetworkChange,
  networks = CARD_CCTP_SOURCE_NETWORKS,
  switching = false,
  disabled = false,
  className,
}: CardNetworkSelectProps) {
  const selected = findCardCctpSourceNetwork(networks, value) ?? networks[0];

  return (
    <div
      className={cn('space-y-2 rounded-xl border border-border/30 bg-background/50 p-4', className)}
      data-testid="card-network-select"
    >
      <div className="flex items-baseline justify-between gap-3">
        <Label htmlFor="card-top-up-network" className="text-sm font-semibold">
          Source network
        </Label>
        <span
          className="font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-primary"
          data-testid="card-network-domain"
        >
          CCTP domain {selected.circleDomain}
        </span>
      </div>

      <select
        id="card-top-up-network"
        value={selected.id}
        disabled={disabled || switching}
        aria-describedby="card-network-hint"
        onChange={(event) => {
          const next = findCardCctpSourceNetwork(networks, event.target.value);
          if (next) onNetworkChange(next);
        }}
        className={cn(
          'min-h-11 w-full rounded-xl border border-input bg-background px-3 py-2 text-sm font-medium',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
          'disabled:cursor-not-allowed disabled:opacity-60',
        )}
        data-testid="card-network-picker"
      >
        {networks.map((network) => (
          <option
            key={network.id}
            value={network.id}
            data-testid={`card-network-option-${network.id}`}
          >
            {network.executable ? network.label : `${network.label} — corridor not enabled`}
          </option>
        ))}
      </select>

      <p
        id="card-network-hint"
        className={cn(
          'text-xs',
          selected.executable ? 'text-muted-foreground' : 'font-medium text-amber-600 dark:text-amber-400',
        )}
        role={selected.executable ? undefined : 'status'}
        aria-live={selected.executable ? undefined : 'polite'}
        data-testid="card-network-notice"
      >
        {switching
          ? `Switching your wallet to ${selected.label}…`
          : selected.executable
            ? `Funds USDC on ${selected.label} through Circle CCTP. Switching networks asks your wallet to change chains.`
            : `${selected.label} is not wired into the CCTP bridge yet, so this top-up cannot execute on it. Pick ${CARD_CCTP_SOURCE_NETWORKS.find((network) => network.executable)?.label ?? 'an enabled network'} to complete a bridge.`}
      </p>
    </div>
  );
}

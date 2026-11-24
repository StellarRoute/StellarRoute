'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ArrowRight } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { WalletAdapterError, getAdapter } from '@/lib/wallet/adapters';
import {
  eip1193Request,
  getInjectedEip1193Provider,
} from '@/lib/wallet/adapters/evm/provider';
import { cn } from '@/lib/utils';
import {
  CardNetworkSelect,
  CARD_CCTP_SOURCE_NETWORKS,
  DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID,
  cardCctpSourceNetworkForCaip2,
  findCardCctpSourceNetwork,
  type CardCctpSourceNetwork,
} from './CardNetworkSelect';

const USDC_INPUT_PATTERN = /^\d+(\.\d{1,7})?$/;
const EVM_ADDRESS_PATTERN = /^0x[0-9a-fA-F]{40}$/;
const BALANCE_OF_SELECTOR = '70a08231';
const USDC_DECIMALS = 6;
const NATIVE_DECIMALS = 18;
/**
 * Ceiling used to turn a gas price into a preview fee for USDC approve +
 * `deposit_for_burn`. Display only — the authoritative pre-flight gas check
 * belongs to the bridge flow, not this modal.
 */
const CCTP_TOP_UP_GAS_UNITS_CEILING = 300_000n;
const DEFAULT_EVM_ADAPTER_ID = 'evm-injected';

function parseUsdcAmount(raw: string): number | null {
  const trimmed = raw.trim();
  if (!USDC_INPUT_PATTERN.test(trimmed)) return null;
  const value = Number(trimmed);
  return Number.isFinite(value) && value > 0 ? value : null;
}

/** ERC-20 `balanceOf` calldata for `address`, padded to a 32-byte word. */
function balanceOfCalldata(address: string): string {
  return `0x${BALANCE_OF_SELECTOR}${address.slice(2).toLowerCase().padStart(64, '0')}`;
}

function formatTokenValue(wei: bigint, decimals: number): string {
  if (wei < 0n) return '0';
  const base = 10n ** BigInt(decimals);
  const whole = wei / base;
  const fraction = wei % base;
  if (fraction === 0n) return whole.toString();
  const digits = fraction
    .toString()
    .padStart(decimals, '0')
    .slice(0, 6)
    .replace(/0+$/, '');
  return digits ? `${whole}.${digits}` : whole.toString();
}

function toBigInt(value: unknown, fallback: bigint): bigint {
  if (typeof value === 'string' && /^0x[0-9a-fA-F]+$/.test(value)) {
    return BigInt(value);
  }
  return fallback;
}

export type CardTopUpSourceState =
  | { status: 'loading' }
  | { status: 'unavailable'; message: string }
  | {
      status: 'ready';
      usdc: string;
      gasFundsEth: string;
      gasPriceGwei: string;
      maxGasFeeEth: string;
      gasShort: boolean;
    }
  | { status: 'error'; message: string };

const UNAVAILABLE = (message: string): CardTopUpSourceState => ({
  status: 'unavailable',
  message,
});

/**
 * Reads the source USDC balance and gas picture for `network` straight from
 * the injected EIP-1193 provider. Every value is re-fetched after a chain
 * switch, so a stale result can never outlive the network it belonged to.
 */
export async function readCardTopUpSourceState(
  network: CardCctpSourceNetwork,
  address: string
): Promise<CardTopUpSourceState> {
  if (!network.usdcAddress) {
    return UNAVAILABLE(
      `Balance and gas preview is unavailable on ${network.label} until the bridge corridor is enabled.`
    );
  }
  if (!EVM_ADDRESS_PATTERN.test(address)) {
    return UNAVAILABLE('Connect an EVM wallet to preview its USDC balance.');
  }

  const provider = getInjectedEip1193Provider();
  if (!provider) {
    return UNAVAILABLE('Connect an EVM wallet to preview its USDC balance.');
  }

  try {
    const chainIdHex = await eip1193Request<string>(provider, 'eth_chainId');
    if (toBigInt(chainIdHex, -1n) !== BigInt(network.chainId)) {
      return UNAVAILABLE(
        `Your wallet is not on ${network.label} yet. Switch networks to refresh this balance.`
      );
    }

    const [rawUsdc, rawGasFunds, rawGasPrice] = await Promise.all([
      eip1193Request<string>(provider, 'eth_call', [
        {
          to: network.usdcAddress,
          data: balanceOfCalldata(address),
        },
        'latest',
      ]),
      eip1193Request<string>(provider, 'eth_getBalance', [address, 'latest']),
      eip1193Request<string>(provider, 'eth_gasPrice'),
    ]);

    const usdc = formatTokenValue(toBigInt(rawUsdc, 0n), USDC_DECIMALS);
    const gasFundsWei = toBigInt(rawGasFunds, 0n);
    const gasPriceWei = toBigInt(rawGasPrice, 0n);
    const maxGasFeeWei = gasPriceWei * CCTP_TOP_UP_GAS_UNITS_CEILING;

    return {
      status: 'ready',
      usdc,
      gasFundsEth: formatTokenValue(gasFundsWei, NATIVE_DECIMALS),
      gasPriceGwei: formatTokenValue(gasPriceWei, 9),
      maxGasFeeEth: formatTokenValue(maxGasFeeWei, NATIVE_DECIMALS),
      gasShort: gasFundsWei < maxGasFeeWei,
    };
  } catch (err) {
    return {
      status: 'error',
      message:
        err instanceof Error && err.message
          ? `Could not read ${network.label}: ${err.message}`
          : `Could not read ${network.label}. Try again.`,
    };
  }
}

export interface CardTopUpModalProps {
  open: boolean;
  /** Raw text from the amount field, validated by the page on confirm. */
  amount: string;
  /** Amount validation error owned by the page. */
  error: string | null;
  onAmountChange: (raw: string) => void;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
  /**
   * EVM address to read the source balance for. When omitted the modal passively
   * reads the connected EVM session (never prompts a connection).
   */
  sourceAddress?: string;
  /** Override the selectable CCTP source networks. */
  networks?: readonly CardCctpSourceNetwork[];
  /** Adapter that owns the wallet network switch request. */
  evmAdapterId?: string;
  /**
   * Hands the chosen network — and with it `circleDomain`, the CCTP source
   * domain a burn is initiated against — to the bridge flow that runs
   * `useCctpSaga`.
   */
  onSourceNetworkChange?: (network: CardCctpSourceNetwork) => void;
  className?: string;
}

/**
 * CARD-41's funding modal, plus CARD-47's multi-EVM source network selector.
 *
 * The amount field, the confirm action and the "Continue to bridge" link behave
 * exactly as they did before network selection existed: with the default
 * network selected the bridge href is byte-for-byte what it was.
 */
export function CardTopUpModal({
  open,
  amount,
  error,
  onAmountChange,
  onOpenChange,
  onConfirm,
  sourceAddress,
  networks = CARD_CCTP_SOURCE_NETWORKS,
  evmAdapterId = DEFAULT_EVM_ADAPTER_ID,
  onSourceNetworkChange,
  className,
}: CardTopUpModalProps) {
  const [selectedId, setSelectedId] = useState(DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID);
  const [sessionAddress, setSessionAddress] = useState<string | null>(null);
  const [switching, setSwitching] = useState(false);
  const [switchMessage, setSwitchMessage] = useState<string | null>(null);
  const [source, setSource] = useState<CardTopUpSourceState | null>(null);
  const [readTick, setReadTick] = useState(0);
  const readSeq = useRef(0);

  const selected = useMemo(
    () => findCardCctpSourceNetwork(networks, selectedId) ?? networks[0],
    [networks, selectedId]
  );
  const address = sourceAddress ?? sessionAddress;

  // Adopt the wallet's real chain and address when the modal opens, so the
  // selector starts from the previously active network rather than a guess.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    const adapter = getAdapter(evmAdapterId);
    if (!adapter) return;

    adapter
      .getSession()
      .then((session) => {
        if (cancelled || !session?.isConnected) return;
        setSessionAddress(session.account.address);
        const onWalletChain = cardCctpSourceNetworkForCaip2(
          networks,
          session.network
        );
        if (onWalletChain) setSelectedId(onWalletChain.id);
      })
      .catch(() => {
        /* Passive read only — absence of a session is not an error. */
      });

    return () => {
      cancelled = true;
    };
  }, [open, evmAdapterId, networks]);

  // Re-query balance + gas on every network switch (AC3). `readTick` makes the
  // refresh fire even when the wallet settles back on the same listed network.
  useEffect(() => {
    if (!open) {
      setSource(null);
      return;
    }
    if (!address || !selected) {
      setSource(null);
      return;
    }

    const target = selected;
    const seq = readSeq.current + 1;
    readSeq.current = seq;
    setSource({ status: 'loading' });

    readCardTopUpSourceState(target, address).then((next) => {
      if (readSeq.current === seq) setSource(next);
    });
  }, [open, address, selected, readTick]);

  const handleNetworkChange = useCallback(
    async (next: CardCctpSourceNetwork) => {
      if (next.id === selectedId || switching) return;
      const previous = findCardCctpSourceNetwork(networks, selectedId);
      const adapter = getAdapter(evmAdapterId);

      setSwitching(true);
      setSwitchMessage(null);
      setSelectedId(next.id);

      if (!adapter?.switchNetwork) {
        setSelectedId(previous?.id ?? DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID);
        setSwitchMessage(
          'Your EVM wallet cannot switch networks from the browser. Change chain in the wallet itself.'
        );
        setSwitching(false);
        return;
      }

      try {
        const info = await adapter.switchNetwork(next.network);
        const confirmed = cardCctpSourceNetworkForCaip2(networks, info.network);
        setReadTick((tick) => tick + 1);
        if (confirmed) {
          setSelectedId(confirmed.id);
          onSourceNetworkChange?.(confirmed);
          if (info.network === previous?.network) {
            setSwitchMessage(
              `The wallet stayed on ${previous?.label}. Top-ups still come from there.`
            );
          }
          return;
        }
        setSelectedId(previous?.id ?? DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID);
        setSwitchMessage(
          `${next.label} is not a network this wallet returned. Reverted to ${previous?.label}.`
        );
      } catch (err) {
        // AC4: a declined switch must leave the selector on the old chain.
        setSelectedId(previous?.id ?? DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID);
        setSwitchMessage(
          err instanceof WalletAdapterError && err.code === 'user_rejected'
            ? `Network switch cancelled — staying on ${previous?.label}.`
            : err instanceof Error && err.message
              ? err.message
              : 'The wallet rejected the network switch.'
        );
      } finally {
        setSwitching(false);
      }
    },
    [evmAdapterId, networks, onSourceNetworkChange, selectedId, switching]
  );

  const parsed = parseUsdcAmount(amount);
  const bridgeQuery = new URLSearchParams();
  if (parsed !== null) bridgeQuery.set('amount', String(parsed));
  bridgeQuery.set('destination', 'card');
  // Only a non-default network changes the href, so the link a live user has
  // always followed stays identical.
  if (selected && selected.id !== DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID) {
    bridgeQuery.set('source_chain', selected.network);
    bridgeQuery.set('source_domain', String(selected.circleDomain));
  }
  const bridgeHref = `/cross-chain-swap?${bridgeQuery.toString()}`;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className={cn(
          'flex w-[min(100%,90vw)] max-h-[min(90dvh,90vh)] flex-col gap-0 overflow-hidden p-0 rounded-2xl border-border/40 bg-background/95 backdrop-blur-xl sm:max-w-[480px] sm:rounded-[28px]',
          className
        )}
        data-testid="card-top-up-modal"
      >
        <div className="min-h-0 flex-1 space-y-4 overflow-y-auto overscroll-contain p-5 sm:p-7">
          <DialogHeader className="space-y-2 text-left">
            <DialogTitle className="font-display text-2xl font-bold tracking-tight">
              Bridge top-up
            </DialogTitle>
            <DialogDescription className="text-pretty text-muted-foreground">
              Fund your card from another chain. The amount below was chosen
              to clear the low-balance alert; edit it if you want to top up
              more, then continue in the bridge flow.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-2 rounded-xl border border-border/30 bg-background/50 p-4">
            <Label htmlFor="card-top-up-amount" className="text-sm font-semibold">
              Top-up amount (USDC)
            </Label>
            <Input
              id="card-top-up-amount"
              type="text"
              inputMode="decimal"
              value={amount}
              aria-invalid={error ? true : undefined}
              aria-describedby={error ? 'card-top-up-amount-error' : undefined}
              onChange={(event) => onAmountChange(event.target.value)}
              data-testid="card-top-up-amount-input"
            />
            {error && (
              <p
                id="card-top-up-amount-error"
                role="alert"
                className="text-xs font-medium text-destructive"
                data-testid="card-top-up-amount-error"
              >
                {error}
              </p>
            )}
          </div>

          {selected && (
            <CardNetworkSelect
              value={selected.id}
              onNetworkChange={handleNetworkChange}
              networks={networks}
              switching={switching}
              className="mt-2"
            />
          )}

          {switchMessage && (
            <p
              role="status"
              aria-live="polite"
              className="text-xs font-medium text-amber-600 dark:text-amber-400"
              data-testid="card-network-switch-message"
            >
              {switchMessage}
            </p>
          )}

          <div
            className="space-y-1.5 rounded-xl border border-border/30 bg-background/50 p-4 text-sm"
            data-testid="card-top-up-source-summary"
          >
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              {selected?.label ?? 'Source'} available to send
            </p>
            <div
              className="flex items-baseline justify-between gap-3"
              data-testid="card-top-up-source-balance"
            >
              <span className="text-muted-foreground">USDC balance</span>
              <span className="font-mono font-semibold">
                {source?.status === 'ready'
                  ? `${source.usdc} USDC`
                  : source?.status === 'loading'
                    ? 'Reading…'
                    : '—'}
              </span>
            </div>
            <div
              className="flex items-baseline justify-between gap-3"
              data-testid="card-top-up-source-gas"
            >
              <span className="text-muted-foreground">Gas estimate</span>
              <span className="font-mono font-semibold">
                {source?.status === 'ready'
                  ? `≤ ${source.maxGasFeeEth} ETH (have ${source.gasFundsEth})`
                  : source?.status === 'loading'
                    ? 'Reading…'
                    : '—'}
              </span>
            </div>
            {source?.status === 'ready' && source.gasShort && (
              <p
                role="alert"
                className="text-xs font-medium text-destructive"
                data-testid="card-top-up-source-gas-short"
              >
                Not enough ETH on {selected?.label} to cover the bridge gas.
              </p>
            )}
            {(source?.status === 'unavailable' ||
              source?.status === 'error') && (
              <p
                className="text-xs text-muted-foreground"
                data-testid="card-top-up-source-status"
              >
                {source.message}
              </p>
            )}
            {source === null && !address && (
              <p
                className="text-xs text-muted-foreground"
                data-testid="card-top-up-source-status"
              >
                Connect an EVM wallet to preview its USDC balance and gas.
              </p>
            )}
          </div>
        </div>

        <DialogFooter className="shrink-0 flex-col gap-2 border-t border-border/20 bg-muted/10 p-4 sm:flex-col sm:p-6">
          <Button
            type="button"
            className="min-h-11 w-full font-semibold"
            onClick={onConfirm}
            data-testid="card-top-up-confirm"
          >
            Confirm top-up amount
          </Button>
          <a
            href={bridgeHref}
            className="inline-flex min-h-11 w-full items-center justify-center gap-2 rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-medium text-slate-700 transition hover:border-slate-400 hover:text-slate-900 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:border-slate-500 dark:hover:text-slate-100"
            data-testid="card-top-up-bridge-link"
          >
            Continue to bridge
            <ArrowRight className="h-4 w-4" aria-hidden />
          </a>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

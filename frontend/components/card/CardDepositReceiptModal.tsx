'use client';

import { useRef, useEffect, useCallback } from 'react';
import { ExternalLink } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { CopyButton } from '@/components/shared/CopyButton';
import { cn } from '@/lib/utils';
import {
  formatDepositReceipt,
  shortenHash,
  shortenAddress,
  type DepositSagaState,
  type DepositReceipt,
} from '@/lib/card/receipt';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface CardDepositReceiptModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sagaState?: DepositSagaState | null;
  /** Pre-computed receipt — if provided, skips calling formatDepositReceipt */
  receipt?: DepositReceipt | null;
  onDone?: () => void;
  className?: string;
}

// ── Sub-components ─────────────────────────────────────────────────────────────

function ReceiptBadge({
  label,
  className,
}: {
  label: string;
  className?: string;
}) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full border border-primary/30 bg-primary/10 px-2.5 py-0.5 font-mono text-[10px] font-semibold uppercase tracking-widest text-primary',
        className,
      )}
    >
      {label}
    </span>
  );
}

function ReceiptDetail({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-border/30 bg-background/50 p-3">
      <dt className="text-[11px] uppercase tracking-wide text-muted-foreground">
        {label}
      </dt>
      <dd className="mt-1 font-semibold">{value}</dd>
    </div>
  );
}

function HashRow({
  label,
  value,
  display,
  copyLabel,
  href,
  explorerLabel,
}: {
  label: string;
  value: string;
  display: string;
  copyLabel: string;
  href?: string;
  explorerLabel?: string;
}) {
  return (
    <div className="rounded-xl border border-border/30 bg-background/50 px-3 py-2.5">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 space-y-0.5">
          <p className="text-[11px] uppercase tracking-wide text-muted-foreground">
            {label}
          </p>
          <p className="break-all font-mono text-xs font-medium" title={value}>
            {display}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <CopyButton value={value} label={copyLabel} />
          {href && (
            <a
              href={href}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex h-8 items-center gap-1 rounded-lg px-2 text-xs font-medium text-primary hover:bg-primary/10"
              aria-label={explorerLabel ?? 'Open in explorer'}
            >
              <ExternalLink className="h-3.5 w-3.5" aria-hidden />
              <span className="hidden sm:inline">
                {explorerLabel ?? 'Explorer'}
              </span>
            </a>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Modal ──────────────────────────────────────────────────────────────────────

/**
 * CARD-51 – Cross-Chain Top-Up Proof of Deposit Receipt Modal.
 *
 * Renders both source EVM burn hash and destination Stellar mint hash with
 * block explorer links.  Displays deposit timestamp, bridged USDC amount, and
 * source network badge.  Accessible from completed top-up activity history
 * items.  Includes a "Copy Receipt Data" button that copies a clean
 * Markdown/text summary to the clipboard.  Print-friendly via @media print.
 */
export function CardDepositReceiptModal({
  open,
  onOpenChange,
  sagaState,
  receipt: receiptProp,
  onDone,
  className,
}: CardDepositReceiptModalProps) {
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (open) {
      closeRef.current?.focus();
    }
  }, [open]);

  const receipt: DepositReceipt = receiptProp ?? formatDepositReceipt({ sagaState });

  const handlePrint = useCallback(() => {
    window.print();
  }, []);

  const hasHashes = receipt.burnTxHash || receipt.mintTxHash;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className={cn(
          'flex w-[min(100%,90vw)] max-h-[min(90dvh,90vh)] flex-col gap-0 overflow-hidden p-0',
          'border-border/40 bg-background/95 backdrop-blur-xl',
          'rounded-2xl sm:rounded-[28px] shadow-2xl sm:max-w-[480px]',
          // Print: remove the overlay chrome so only the receipt body renders
          'print:shadow-none print:border-none print:max-h-none print:overflow-visible',
          className,
        )}
        data-testid="card-deposit-receipt-modal"
      >
        {/* ── Scrollable body ── */}
        <div className="min-h-0 flex-1 space-y-5 overflow-y-auto overscroll-contain p-5 sm:p-7 print:overflow-visible">
          <DialogHeader className="space-y-3 text-center sm:text-center">
            <ReceiptBadge label="Top-Up receipt" />
            <DialogTitle className="font-display text-2xl font-bold tracking-tight">
              Cross-Chain Deposit Proof
            </DialogTitle>
            <DialogDescription className="text-pretty text-muted-foreground">
              {hasHashes
                ? 'Both on-chain transaction hashes are confirmed. Open the destination explorer link to verify the USDC mint.'
                : 'Transaction hashes are not yet available for this status. Use the source network badge and timestamp to locate the transfer.'}
            </DialogDescription>
          </DialogHeader>

          {/* Amount + network details */}
          <dl className="grid gap-2 text-sm sm:grid-cols-2">
            <ReceiptDetail
              label="Bridged Amount"
              value={receipt.usdcAmount}
            />
            <ReceiptDetail
              label="Status"
              value={receipt.status}
            />
          </dl>

          {/* Source network badge row */}
          <div className="rounded-xl border border-border/30 bg-background/50 px-3 py-2.5">
            <dl className="grid gap-1 sm:grid-cols-2">
              <div>
                <dt className="text-[11px] uppercase tracking-wide text-muted-foreground">
                  Source Network
                </dt>
                <dd className="mt-1 flex items-center gap-1.5">
                  <ReceiptBadge
                    label={receipt.sourceNetworkBadge}
                    className="text-[10px]"
                  />
                </dd>
              </div>
              <div>
                <dt className="text-[11px] uppercase tracking-wide text-muted-foreground">
                  Destination Network
                </dt>
                <dd className="mt-1 flex items-center gap-1.5">
                  <ReceiptBadge label="Stellar Testnet" className="text-[10px]" />
                </dd>
              </div>
            </dl>
          </div>

          {/* Deposit timestamp */}
          <div className="rounded-xl border border-border/30 bg-background/50 p-3">
            <dt className="text-[11px] uppercase tracking-wide text-muted-foreground">
              Deposit Timestamp
            </dt>
            <dd
              className="mt-1 font-mono text-sm font-medium"
              data-testid="receipt-timestamp"
            >
              {receipt.timestampFormatted}
            </dd>
          </div>

          {/* Recipient address */}
          {receipt.recipient && (
            <HashRow
              label="Recipient Address"
              value={receipt.recipient}
              display={shortenAddress(receipt.recipient)}
              copyLabel="Copy recipient address"
            />
          )}

          {/* Source EVM burn tx */}
          {receipt.burnTxHash ? (
            <HashRow
              label="Source EVM Burn Tx"
              value={receipt.burnTxHash}
              display={shortenHash(receipt.burnTxHash)}
              copyLabel="Copy source burn transaction hash"
              href={receipt.sourceExplorerUrl}
              explorerLabel="View burn"
            />
          ) : (
            <p
              className="text-xs text-muted-foreground"
              role="status"
              data-testid="receipt-no-burn-hash"
            >
              Source burn transaction hash not yet available.
            </p>
          )}

          {/* Destination Stellar mint tx */}
          {receipt.mintTxHash ? (
            <HashRow
              label="Destination Stellar Mint Tx"
              value={receipt.mintTxHash}
              display={shortenHash(receipt.mintTxHash)}
              copyLabel="Copy destination mint transaction hash"
              href={receipt.destinationExplorerUrl}
              explorerLabel="View mint"
            />
          ) : (
            <p
              className="text-xs text-muted-foreground"
              role="status"
              data-testid="receipt-no-mint-hash"
            >
              Destination mint transaction hash not yet available.
            </p>
          )}
        </div>

        {/* ── Footer actions — hidden during print ── */}
        <DialogFooter
          className={cn(
            'shrink-0 flex-col gap-2 border-t border-border/20 bg-muted/10 p-4 sm:flex-col sm:p-6',
            'print:hidden',
          )}
        >
          <CopyButton
            value={receipt.markdownSummary}
            label="Copy Receipt Data"
          />
          {/* Render a more prominent "Copy Receipt Data" button */}
          <Button
            type="button"
            variant="outline"
            className="min-h-11 w-full"
            onClick={() => {
              navigator.clipboard?.writeText(receipt.markdownSummary).catch(() => {});
            }}
            data-testid="card-deposit-receipt-copy"
          >
            Copy Receipt Data
          </Button>

          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="w-full gap-1.5"
            onClick={handlePrint}
            aria-label="Print or save receipt as PDF"
            data-testid="card-deposit-receipt-print"
          >
            {/* Inline printer icon — avoids adding to the lucide-react mock */}
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden
              className="h-4 w-4 shrink-0"
            >
              <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
              <path d="M6 9V3a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v6" />
              <rect x="6" y="14" width="12" height="8" rx="1" />
            </svg>
            Print / Save PDF
          </Button>

          <Button
            ref={closeRef}
            className="min-h-11 w-full font-semibold"
            onClick={() => {
              onOpenChange(false);
              onDone?.();
            }}
            data-testid="card-deposit-receipt-done"
          >
            Done
          </Button>

          <Button
            type="button"
            variant="outline"
            className="min-h-11 w-full"
            onClick={() => onOpenChange(false)}
            data-testid="card-deposit-receipt-keep-open"
          >
            Keep receipt on screen
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

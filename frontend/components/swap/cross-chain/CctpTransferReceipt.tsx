'use client';

import { ExternalLink } from 'lucide-react';
import { CopyButton } from '@/components/shared/CopyButton';
import type {
  CctpDirection,
  CctpQuoteResponse,
  CctpTransferStatusResponse,
} from '@/lib/cctp/types';
import { cn } from '@/lib/utils';
import { cctpExplorerUrl, shortenHash, shortenAddress } from '@/lib/cctp/receipt';

export interface CctpTransferReceiptProps {
  quote: CctpQuoteResponse | null;
  transferStatus: CctpTransferStatusResponse | null;
  recipient?: string | null;
  className?: string;
  compact?: boolean;
}

export function CctpTransferReceipt({
  quote,
  transferStatus,
  recipient,
  className,
  compact = false,
}: CctpTransferReceiptProps) {
  const direction =
    transferStatus?.direction ?? quote?.direction ?? 'stellar_to_evm';
  const amountSent = quote?.source_amount;
  const amountReceived = quote?.destination_amount;
  const sourceHash = transferStatus?.source_tx_hash;
  const destHash = transferStatus?.destination_tx_hash;
  const supportRef =
    transferStatus?.support_reference_id ?? transferStatus?.transfer_id;
  const destLabel =
    direction === 'stellar_to_evm' ? 'Destination (EVM)' : 'Destination (Stellar)';

  return (
    <div
      className={cn(
        'space-y-3 rounded-2xl border border-primary/30 bg-primary/5 p-4',
        className,
      )}
      data-testid="cctp-transfer-receipt"
    >
      {!compact && (
        <div className="space-y-1">
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-primary">
            Transfer receipt
          </p>
          <p className="text-sm text-muted-foreground">
            Verify the mint on the destination explorer — you should not need to
            hunt through wallet history.
          </p>
        </div>
      )}

      <dl className="grid gap-2 text-sm sm:grid-cols-2">
        <ReceiptDetail
          label="Sent"
          value={amountSent ? `${amountSent} USDC` : 'USDC'}
        />
        <ReceiptDetail
          label="Received"
          value={amountReceived ? `${amountReceived} USDC` : 'USDC'}
        />
      </dl>

      {recipient && (
        <HashRow
          label={destLabel}
          value={recipient}
          display={shortenAddress(recipient)}
          copyLabel="Copy destination address"
        />
      )}

      {sourceHash && (
        <HashRow
          label="Source burn tx"
          value={sourceHash}
          display={shortenHash(sourceHash)}
          copyLabel="Copy source transaction hash"
          href={cctpExplorerUrl(sourceHash, 'source', direction)}
          explorerLabel="View burn"
        />
      )}

      {destHash && (
        <HashRow
          label="Destination mint tx"
          value={destHash}
          display={shortenHash(destHash)}
          copyLabel="Copy destination transaction hash"
          href={cctpExplorerUrl(destHash, 'dest', direction)}
          explorerLabel="View mint"
        />
      )}

      {!sourceHash && !destHash && (
        <p className="text-xs text-muted-foreground" role="status">
          Transaction hashes are not on this status payload yet. Use the support
          reference below if you need help locating the mint.
        </p>
      )}

      {supportRef && (
        <HashRow
          label="Support reference"
          value={supportRef}
          display={shortenHash(supportRef, 10, 8)}
          copyLabel="Copy support reference"
        />
      )}
    </div>
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
          <p className="font-mono text-xs font-medium break-all" title={value}>
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
              <span className="hidden sm:inline">{explorerLabel ?? 'Explorer'}</span>
            </a>
          )}
        </div>
      </div>
    </div>
  );
}

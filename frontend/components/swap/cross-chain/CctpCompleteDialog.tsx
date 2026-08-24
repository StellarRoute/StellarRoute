'use client';

import { useEffect, useRef } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import type { CctpQuoteResponse, CctpTransferStatusResponse } from '@/lib/cctp/types';
import { CctpJourneyVisual } from './CctpJourneyVisual';
import { CctpTransferReceipt } from './CctpTransferReceipt';

export interface CctpCompleteDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  quote: CctpQuoteResponse | null;
  transferStatus: CctpTransferStatusResponse | null;
  recipient?: string | null;
  onDone: () => void;
}

export function CctpCompleteDialog({
  open,
  onOpenChange,
  quote,
  transferStatus,
  recipient,
  onDone,
}: CctpCompleteDialogProps) {
  const doneRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (open) {
      doneRef.current?.focus();
    }
  }, [open]);

  const destHash = transferStatus?.destination_tx_hash;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="flex w-[min(100%,90vw)] max-h-[min(90dvh,90vh)] flex-col gap-0 overflow-hidden p-0 border-border/40 bg-background/95 backdrop-blur-xl rounded-2xl sm:rounded-[28px] shadow-2xl sm:max-w-[460px]"
        data-testid="cctp-complete-dialog"
      >
        <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain p-5 sm:p-7 space-y-5">
          <DialogHeader className="space-y-3 text-center sm:text-center">
            <p className="font-mono text-[11px] uppercase tracking-[0.22em] text-primary">
              Bridge complete
            </p>
            <DialogTitle className="font-display text-2xl font-bold tracking-tight">
              USDC landed on destination
            </DialogTitle>
            <DialogDescription className="text-muted-foreground text-pretty">
              {destHash
                ? 'Open the destination mint below to confirm the balance without digging through your wallet.'
                : 'Copy the destination address and support reference, then confirm the mint on the destination explorer.'}
            </DialogDescription>
          </DialogHeader>

          <CctpJourneyVisual status="completed" />

          <CctpTransferReceipt
            quote={quote}
            transferStatus={transferStatus}
            recipient={recipient}
            compact
          />
        </div>

        <DialogFooter className="shrink-0 flex-col gap-2 border-t border-border/20 bg-muted/10 p-4 sm:p-6 sm:flex-col">
          <Button
            ref={doneRef}
            className="min-h-11 w-full font-semibold"
            onClick={() => {
              onOpenChange(false);
              onDone();
            }}
            data-testid="cctp-complete-done"
          >
            Done — start new transfer
          </Button>
          <Button
            type="button"
            variant="outline"
            className="min-h-11 w-full"
            onClick={() => onOpenChange(false)}
            data-testid="cctp-complete-keep-open"
          >
            Keep receipt on screen
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

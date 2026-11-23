'use client';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { cn } from '@/lib/utils';
import { getIndicativeRatesMatrix } from '@/lib/card/fx';

export interface CardFxMatrixModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  usdcAmount: number;
  className?: string;
}

export function CardFxMatrixModal({
  open,
  onOpenChange,
  usdcAmount,
  className,
}: CardFxMatrixModalProps) {
  const matrix = getIndicativeRatesMatrix({ usdcAmount });
  const hasAmount = matrix.usdcAmount > 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className={cn(
          'rounded-2xl border-border/40 bg-background/95 backdrop-blur-xl sm:max-w-[520px]',
          className,
        )}
        data-testid="card-fx-matrix-modal"
      >
        <DialogHeader className="space-y-2 text-left">
          <DialogTitle className="font-display text-2xl font-bold tracking-tight">
            Currency comparison
          </DialogTitle>
          <DialogDescription className="text-pretty text-muted-foreground">
            What {matrix.usdcAmount} USDC converts to across supported card
            currencies.
          </DialogDescription>
        </DialogHeader>

        <Badge
          variant="outline"
          className="w-fit border-amber-300 bg-amber-50 text-amber-800 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-200"
          data-testid="card-fx-matrix-disclaimer"
        >
          indicative: true · fixture rates dated {matrix.fixtureDate}
        </Badge>

        {!hasAmount && (
          <p
            role="status"
            className="text-xs text-muted-foreground"
            data-testid="card-fx-matrix-empty"
          >
            Enter an amount greater than 0 USDC to compare currencies.
          </p>
        )}

        <Table data-testid="card-fx-matrix-table">
          <TableCaption className="sr-only">
            Indicative conversion of {matrix.usdcAmount} USDC
          </TableCaption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">Currency</TableHead>
              <TableHead scope="col">Rate</TableHead>
              <TableHead scope="col" className="text-right">
                Amount
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {matrix.rows.map((row) => (
              <TableRow
                key={row.fiatCurrency}
                data-testid={`card-fx-matrix-row-${row.fiatCurrency}`}
              >
                <TableCell className="font-medium">
                  <span aria-hidden="true" className="mr-2">
                    {row.flag}
                  </span>
                  {row.fiatCurrency}
                </TableCell>
                <TableCell className="text-muted-foreground">
                  1 USDC = {row.ratePerUsdc} {row.fiatCurrency}
                </TableCell>
                <TableCell
                  className="text-right font-mono"
                  data-testid={`card-fx-matrix-amount-${row.fiatCurrency}`}
                >
                  {row.convertedAmount}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </DialogContent>
    </Dialog>
  );
}

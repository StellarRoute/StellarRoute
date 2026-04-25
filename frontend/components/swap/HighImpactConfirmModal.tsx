'use client';

import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { AlertCircle, ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";

interface HighImpactConfirmModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  priceImpact: number;
  fromAmount: string;
  fromSymbol: string;
  toAmount: string;
  toSymbol: string;
}

export function HighImpactConfirmModal({
  isOpen,
  onClose,
  onConfirm,
  priceImpact,
  fromAmount,
  fromSymbol,
  toAmount,
  toSymbol,
}: HighImpactConfirmModalProps) {
  const [isChecked, setIsChecked] = useState(false);

  const handleConfirm = () => {
    if (isChecked) {
      onConfirm();
      onClose();
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[420px] p-0 overflow-hidden border-border/40 bg-background/95 backdrop-blur-xl rounded-[32px] shadow-2xl">
        <div className="p-8 space-y-6">
          <DialogHeader>
            <div className="mx-auto w-16 h-16 bg-destructive/10 rounded-full flex items-center justify-center mb-4 animate-pulse">
              <AlertCircle className="h-8 w-8 text-destructive" />
            </div>
            <DialogTitle className="text-2xl font-bold text-center tracking-tight">High Price Impact</DialogTitle>
            <DialogDescription className="text-center text-muted-foreground pt-2">
              This trade has a high price impact and may result in a significant loss of value.
            </DialogDescription>
          </DialogHeader>

          <div className="bg-muted/30 rounded-2xl p-6 border border-border/20 space-y-4">
            <div className="flex items-center justify-between text-sm font-medium">
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">Swap</span>
                <span className="text-foreground">{fromAmount} {fromSymbol}</span>
              </div>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">For</span>
                <span className="text-foreground">{parseFloat(toAmount).toFixed(4)} {toSymbol}</span>
              </div>
            </div>

            <div className="pt-4 border-t border-border/20 flex justify-between items-center">
              <span className="text-sm font-semibold text-muted-foreground">Price Impact</span>
              <span className="text-lg font-black text-destructive tabular-nums">
                {priceImpact.toFixed(2)}%
              </span>
            </div>
          </div>

          <div 
            className="flex items-start space-x-3 p-4 bg-muted/20 rounded-xl cursor-pointer hover:bg-muted/30 transition-colors"
            onClick={() => setIsChecked(!isChecked)}
          >
            <Checkbox 
              id="risk-confirm" 
              checked={isChecked} 
              onCheckedChange={(checked) => setIsChecked(checked === true)}
              className="mt-1"
            />
            <label
              htmlFor="risk-confirm"
              className="text-sm font-medium leading-relaxed select-none cursor-pointer"
            >
              I understand that this trade has a high price impact and I may lose significant funds.
            </label>
          </div>
        </div>

        <DialogFooter className="flex flex-col sm:flex-row gap-3 p-8 bg-muted/10 border-t border-border/20">
          <Button 
            variant="outline" 
            onClick={onClose}
            className="flex-1 h-12 rounded-xl font-bold"
          >
            Cancel
          </Button>
          <Button
            variant="destructive"
            disabled={!isChecked}
            onClick={handleConfirm}
            className={cn(
              "flex-1 h-12 rounded-xl font-bold shadow-lg shadow-destructive/20 transition-all active:scale-95",
              !isChecked && "opacity-50 grayscale"
            )}
          >
            Confirm Swap
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

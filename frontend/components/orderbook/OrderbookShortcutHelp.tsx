'use client';

import React from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

interface OrderbookShortcutHelpProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function OrderbookShortcutHelp({
  open,
  onOpenChange,
}: OrderbookShortcutHelpProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-md max-h-[85vh] overflow-y-auto"
        data-testid="orderbook-shortcut-help"
      >
        <DialogHeader>
          <DialogTitle>Orderbook Shortcuts</DialogTitle>
          <DialogDescription className="sr-only">
            List of keyboard shortcuts available on the orderbook page
          </DialogDescription>
        </DialogHeader>
        <ul className="space-y-3 text-sm pt-2">
          <li className="flex justify-between items-center">
            <span>Open keyboard help</span>
            <kbd className="font-mono bg-muted px-2 py-0.5 rounded text-xs border">
              ?
            </kbd>
          </li>
          <li className="flex justify-between items-center">
            <span>Close modal</span>
            <kbd className="font-mono bg-muted px-2 py-0.5 rounded text-xs border">
              Esc
            </kbd>
          </li>
          <li className="flex justify-between items-center">
            <span>Refresh orderbook & depth</span>
            <kbd className="font-mono bg-muted px-2 py-0.5 rounded text-xs border">
              Alt+R
            </kbd>
          </li>
        </ul>
      </DialogContent>
    </Dialog>
  );
}

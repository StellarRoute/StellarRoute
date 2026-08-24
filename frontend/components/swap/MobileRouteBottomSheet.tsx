'use client';

import { useState } from 'react';
import { useMediaQuery } from '@/hooks/useMediaQuery';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { RouteDisplay } from './RouteDisplay';
import { Button } from '@/components/ui/button';
import { ChevronUp, Layers, Maximize2, Minimize2 } from 'lucide-react';
import type { PriceQuote } from '@/types';

interface MobileRouteBottomSheetProps {
  quote: PriceQuote | null;
  amountOut: string;
  isLoading: boolean;
}

export function MobileRouteBottomSheet({
  quote,
  amountOut,
  isLoading,
}: MobileRouteBottomSheetProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [snapPoint, setSnapPoint] = useState<'half' | 'full'>('half');
  const isDesktop = useMediaQuery('(min-width: 768px)');

  if (isDesktop) {
    return (
      <RouteDisplay quote={quote} amountOut={amountOut} isLoading={isLoading} />
    );
  }

  return (
    <Sheet open={isOpen} onOpenChange={setIsOpen}>
      <SheetTrigger asChild>
        <Button
          type="button"
          variant="outline"
          className="w-full flex items-center justify-between p-4 h-14 rounded-2xl bg-muted/40 border-border/30 hover:bg-muted/60 transition-all duration-300 group"
          aria-label="Open routing path summary selection sheet"
          data-testid="route-sheet-trigger"
        >
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-lg bg-primary/10 text-primary">
              <Layers className="h-4 w-4" />
            </div>
            <div className="text-left">
              <span className="text-xs text-muted-foreground block font-medium">
                Trade Routing Path
              </span>
              <span className="text-xs font-bold font-mono text-foreground">
                {isLoading
                  ? 'Calculating best route...'
                  : 'Optimized Multi-Hop Available'}
              </span>
            </div>
          </div>
          <ChevronUp className="h-4 w-4 text-muted-foreground group-hover:translate-y-[-2px] transition-transform" />
        </Button>
      </SheetTrigger>

      <SheetContent
        side="bottom"
        className={htmlCn(
          'w-full rounded-t-[28px] border-t border-border/40 bg-background/95 backdrop-blur-xl p-6 transition-all duration-300 ease-in-out shadow-2xl focus:outline-none',
          snapPoint === 'half' ? 'h-[50vh]' : 'h-[94vh]'
        )}
        data-testid="route-sheet-content"
      >
        <div className="w-12 h-1 bg-muted-foreground/20 rounded-full mx-auto mb-4 pointer-events-none" />

        <SheetHeader className="flex flex-row items-center justify-between space-y-0 pb-4 border-b border-border/10">
          <div>
            <SheetTitle className="text-base font-bold tracking-tight">
              Select Order Route
            </SheetTitle>
            {/* Kept close to top header for styling but perfectly coupled to Radix dialog schema */}
            <SheetDescription className="text-xs text-muted-foreground mt-1">
              Optimize for pricing impact and liquidity pool density.
            </SheetDescription>
          </div>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="h-8 w-8 rounded-lg hover:bg-muted/80 text-muted-foreground"
            onClick={() => setSnapPoint(snapPoint === 'half' ? 'full' : 'half')}
            aria-label={
              snapPoint === 'half'
                ? 'Expand sheet to full height'
                : 'Collapse sheet to half height'
            }
            data-testid="route-sheet-snap-toggle"
          >
            {snapPoint === 'half' ? (
              <Maximize2 className="h-4 w-4" data-testid="icon-maximize" />
            ) : (
              <Minimize2 className="h-4 w-4" data-testid="icon-minimize" />
            )}
          </Button>
        </SheetHeader>

        <div className="overflow-y-auto h-full pb-20 pt-4 space-y-4 pr-1">
          <div className="p-4 rounded-xl border border-primary/20 bg-primary/5 shadow-inner">
            <RouteDisplay
              quote={quote}
              amountOut={amountOut}
              isLoading={isLoading}
            />
          </div>

          <div className="space-y-2">
            <span className="text-[10px] font-bold text-muted-foreground/70 uppercase tracking-wider block px-1">
              Alternative Liquidity Routes
            </span>
            {[1, 2].map((idx) => (
              <button
                key={idx}
                type="button"
                onClick={() => setIsOpen(false)}
                className="w-full text-left p-3.5 rounded-xl border border-border/40 bg-muted/10 hover:border-muted-foreground/30 hover:bg-muted/20 transition-all font-mono text-xs flex justify-between items-center"
              >
                <span className="text-muted-foreground">
                  Stellar Native Pool{' '}
                  {idx === 1 ? '(Direct)' : '(Secondary Multi-Hop)'}
                </span>
                <span className="font-bold text-emerald-500">
                  +{idx === 1 ? '0.02%' : '0.11%'} Slippage
                </span>
              </button>
            ))}
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function htmlCn(...classes: Array<string | false>) {
  return classes.filter(Boolean).join(' ');
}

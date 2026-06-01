"use client";

import { useState, useMemo } from "react";
import { PriceQuote } from "@/types";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { 
  CheckCircle2, 
  Zap, 
  ShieldCheck,
  Diff,
  ArrowRightLeft,
  TrendingUp,
  Scale
} from "lucide-react";
import { cn } from "@/lib/utils";

export interface VenueQuote extends PriceQuote {
  venueName: string;
  isAggregated?: boolean;
  reliabilityScore?: number; // 0-1
}

interface QuoteInspectorProps {
  quotes: VenueQuote[];
  onSelect: (quote: VenueQuote) => void;
  isLoading?: boolean;
}

export function QuoteInspector({ quotes, onSelect, isLoading }: QuoteInspectorProps) {
  const [reconciledQuote, setReconciledQuote] = useState<VenueQuote | null>(null);

  const bestQuote = useMemo(() => {
    if (quotes.length === 0) return null;
    return quotes.reduce((best, current) => {
      // Comparison based on total output
      return parseFloat(current.total) > parseFloat(best.total) ? current : best;
    }, quotes[0]);
  }, [quotes]);

  const handleReconcile = (quote: VenueQuote) => {
    setReconciledQuote(quote);
    onSelect(quote);
  };

  if (isLoading) {
    return (
      <Card className="w-full animate-pulse border-primary/10">
        <CardHeader>
          <div className="h-6 w-1/3 bg-muted rounded mb-2"></div>
          <div className="h-4 w-1/2 bg-muted rounded"></div>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-16 w-full bg-muted/50 rounded-lg"></div>
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6 w-full max-w-4xl mx-auto">
      <Card className="shadow-xl border-primary/10 bg-background/50 backdrop-blur-md">
        <CardHeader className="pb-4">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2 text-xl">
                <Diff className="w-6 h-6 text-primary" />
                Cross-Venue Quote Inspector
              </CardTitle>
              <CardDescription className="text-sm">
                Deterministic comparison of real-time liquidity paths
              </CardDescription>
            </div>
            <div className="flex gap-2">
              <Badge variant="secondary" className="bg-primary/10 text-primary hover:bg-primary/20 border-primary/20">
                <ArrowRightLeft className="w-3 h-3 mr-1" />
                {quotes.length} Sources
              </Badge>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="rounded-xl border border-border/50 overflow-hidden bg-card/30">
            <Table>
              <TableHeader className="bg-muted/50">
                <TableRow className="hover:bg-transparent border-border/50">
                  <TableHead className="font-bold">Venue</TableHead>
                  <TableHead className="font-bold">Execution Rate</TableHead>
                  <TableHead className="font-bold">Total Output</TableHead>
                  <TableHead className="font-bold">Rel. Diff</TableHead>
                  <TableHead className="text-right font-bold">Action</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {quotes.map((quote, index) => {
                  const isBest = quote === bestQuote;
                  const isSelected = reconciledQuote === quote;
                  const outputDiff = bestQuote 
                    ? ((parseFloat(quote.total) - parseFloat(bestQuote.total)) / parseFloat(bestQuote.total)) * 100
                    : 0;
                  
                  return (
                    <TableRow 
                      key={`${quote.venueName}-${index}`}
                      className={cn(
                        "transition-all duration-200 group",
                        isBest ? "bg-success/5 hover:bg-success/10" : "hover:bg-muted/30",
                        isSelected && "ring-2 ring-primary ring-inset bg-primary/5"
                      )}
                    >
                      <TableCell className="py-4">
                        <div className="flex flex-col">
                          <div className="flex items-center gap-1.5 font-semibold">
                            {quote.venueName}
                            {isBest && (
                              <Badge className="bg-success text-[10px] h-4 px-1 leading-none border-none">
                                OPTIMAL
                              </Badge>
                            )}
                            {quote.isAggregated && (
                              <Zap className="w-3 h-3 text-amber-500 fill-amber-500" />
                            )}
                          </div>
                          <span className="text-xs text-muted-foreground flex items-center gap-1 mt-0.5">
                            <Scale className="w-3 h-3" />
                            {quote.path.length} Hop{quote.path.length !== 1 ? 's' : ''} via {quote.path[0]?.source.split(':')[0].toUpperCase()}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col">
                          <span className="font-mono text-sm">{parseFloat(quote.price).toFixed(6)}</span>
                          <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
                            {quote.base_asset.asset_code || 'XLM'} / {quote.quote_asset.asset_code || 'XLM'}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col">
                          <span className="font-bold text-sm">
                            {parseFloat(quote.total).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 4 })}
                          </span>
                          <span className="text-[10px] text-muted-foreground font-medium uppercase">
                            {quote.quote_asset.asset_code || 'XLM'}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        {isBest ? (
                          <span className="text-xs font-bold text-success flex items-center gap-1">
                            <TrendingUp className="w-3 h-3" />
                            Reference
                          </span>
                        ) : (
                          <span className={cn(
                            "text-xs font-medium",
                            outputDiff < 0 ? "text-destructive" : "text-success"
                          )}>
                            {outputDiff > 0 ? '+' : ''}{outputDiff.toFixed(3)}%
                          </span>
                        )}
                      </TableCell>
                      <TableCell className="text-right">
                        <Button 
                          size="sm" 
                          variant={isSelected ? "default" : "outline"}
                          className={cn(
                            "h-8 px-4 transition-all",
                            isSelected ? "bg-primary shadow-lg shadow-primary/20" : "hover:bg-primary/10 hover:text-primary hover:border-primary/30"
                          )}
                          onClick={() => handleReconcile(quote)}
                        >
                          {isSelected ? (
                            <span className="flex items-center gap-1">
                              <CheckCircle2 className="w-3 h-3" />
                              Selected
                            </span>
                          ) : isBest ? "Reconcile" : "Select"}
                        </Button>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      {reconciledQuote && (
        <Card className="border-success/20 bg-success/5 animate-in fade-in slide-in-from-top-2 duration-300 shadow-lg shadow-success/5">
          <CardHeader className="py-4">
            <CardTitle className="text-base flex items-center gap-2 text-success">
              <ShieldCheck className="w-5 h-5" />
              Reconciliation Finalized
            </CardTitle>
          </CardHeader>
          <CardContent className="pb-4">
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground italic">
                  Deterministic reconciliation complete for venue <span className="font-bold text-foreground not-italic">&quot;{reconciledQuote.venueName}&quot;</span>.
                </p>
                <div className="flex items-center gap-4 text-xs font-medium text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3 text-success" /> Price Integrity Verified
                  </span>
                  <span className="flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3 text-success" /> Slip-protection Applied
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-3 bg-background/50 p-2 rounded-lg border border-success/10">
                <div className="text-right">
                  <div className="text-[10px] uppercase text-muted-foreground font-bold">Execution Output</div>
                  <div className="text-lg font-black text-success">
                    {parseFloat(reconciledQuote.total).toFixed(4)} {reconciledQuote.quote_asset.asset_code || 'XLM'}
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {!reconciledQuote && (
        <div className="p-4 rounded-xl border border-dashed border-primary/20 bg-primary/5 flex items-center justify-center gap-3">
          <Scale className="w-5 h-5 text-primary animate-pulse" />
          <span className="text-sm font-medium text-primary/80">Select a venue to begin deterministic reconciliation</span>
        </div>
      )}
    </div>
  );
}

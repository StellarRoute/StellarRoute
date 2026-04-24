"use client";

import React from "react";
import { format } from "date-fns";
import { 
  CheckCircle2, 
  XCircle, 
  Clock, 
  ExternalLink, 
  Copy, 
  Check,
  ChevronRight,
  Info
} from "lucide-react";

import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { toast } from "sonner";
import { TransactionRecord } from "@/types/transaction";
import { RouteVisualization } from "./RouteVisualization";

import { cn } from "@/lib/utils";

interface TransactionStatusDrawerProps {
  transaction: TransactionRecord | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

export function TransactionStatusDrawer({
  transaction,
  isOpen,
  onOpenChange,
}: TransactionStatusDrawerProps) {
  const [copied, setCopied] = React.useState(false);

  if (!transaction) return null;

  const handleCopyHash = () => {
    if (transaction.hash) {
      navigator.clipboard.writeText(transaction.hash);
      setCopied(true);
      toast.success("Transaction hash copied to clipboard");
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const getStatusIcon = (status: TransactionRecord["status"]) => {
    switch (status) {
      case "success":
        return <CheckCircle2 className="h-8 w-8 text-success" />;
      case "failed":
        return <XCircle className="h-8 w-8 text-destructive" />;
      default:
        return <Clock className="h-8 w-8 text-blue-500 animate-pulse" />;
    }
  };

  const getStatusLabel = (status: TransactionRecord["status"]) => {
    switch (status) {
      case "success":
        return "Transaction Successful";
      case "failed":
        return "Transaction Failed";
      case "pending":
      case "submitting":
      case "processing":
        return "Transaction Processing";
      default:
        return "Transaction Status";
    }
  };

  const explorers = [
    {
      name: "Stellar.expert",
      url: `https://stellar.expert/explorer/public/tx/${transaction.hash}`,
    },
    {
      name: "Lumenscan",
      url: `https://lumenscan.io/txs/${transaction.hash}`,
    },
    {
      name: "Steexp",
      url: `https://steexp.com/tx/${transaction.hash}`,
    },
  ];

  return (
    <Sheet open={isOpen} onOpenChange={onOpenChange}>
      <SheetContent className="w-full sm:max-w-md p-0 flex flex-col h-full border-l border-primary/10 bg-background/95 backdrop-blur-md">
        <ScrollArea className="flex-1">
          <div className="p-6 space-y-8">
            {/* Header / Status */}
            <div className="flex flex-col items-center text-center space-y-4 pt-4">
              <div className="p-3 rounded-full bg-muted/50">
                {getStatusIcon(transaction.status)}
              </div>
              <div className="space-y-1">
                <h2 className="text-xl font-bold tracking-tight">
                  {getStatusLabel(transaction.status)}
                </h2>
                <p className="text-sm text-muted-foreground">
                  {format(transaction.timestamp, "PPP 'at' p")}
                </p>
              </div>
            </div>

            <Separator className="bg-primary/5" />

            {/* Swap Details Card */}
            <div className="rounded-xl border border-primary/10 bg-muted/30 p-5 space-y-4 shadow-sm">
              <div className="flex items-center justify-between">
                <div className="flex flex-col">
                  <span className="text-xs text-muted-foreground uppercase tracking-wider font-semibold">From</span>
                  <div className="flex items-center gap-2 mt-1">
                    <span className="text-lg font-bold">{transaction.fromAmount}</span>
                    <Badge variant="outline" className="font-mono">{transaction.fromAsset}</Badge>
                  </div>
                </div>
                <ChevronRight className="h-5 w-5 text-muted-foreground/30" />
                <div className="flex flex-col items-end text-right">
                  <span className="text-xs text-muted-foreground uppercase tracking-wider font-semibold">To</span>
                  <div className="flex items-center gap-2 mt-1">
                    <Badge variant="outline" className="font-mono">{transaction.toAsset}</Badge>
                    <span className="text-lg font-bold text-success">{transaction.toAmount}</span>
                  </div>
                </div>
              </div>

              <div className="pt-2 grid grid-cols-2 gap-y-3 gap-x-4 border-t border-primary/5 mt-2">
                <div>
                  <span className="text-[10px] text-muted-foreground uppercase block">Rate</span>
                  <span className="text-xs font-medium">1 {transaction.fromAsset} = {transaction.exchangeRate} {transaction.toAsset}</span>
                </div>
                <div className="text-right">
                  <span className="text-[10px] text-muted-foreground uppercase block">Price Impact</span>
                  <span className={cn(
                    "text-xs font-medium",
                    parseFloat(transaction.priceImpact) > 1 ? "text-yellow-500" : "text-foreground"
                  )}>{transaction.priceImpact}</span>
                </div>
                <div>
                  <span className="text-[10px] text-muted-foreground uppercase block">Network Fee</span>
                  <span className="text-xs font-medium">{transaction.networkFee} XLM</span>
                </div>
                <div className="text-right">
                  <span className="text-[10px] text-muted-foreground uppercase block">Minimum Received</span>
                  <span className="text-xs font-medium">{transaction.minReceived} {transaction.toAsset}</span>
                </div>
              </div>
            </div>

            {/* Error Message if failed */}
            {transaction.status === "failed" && transaction.errorMessage && (
              <div className="rounded-lg border border-destructive/20 bg-destructive/5 p-4 flex gap-3">
                <Info className="h-5 w-5 text-destructive shrink-0" />
                <div className="space-y-1">
                  <p className="text-sm font-semibold text-destructive">Error Details</p>
                  <p className="text-xs text-destructive/80 leading-relaxed">{transaction.errorMessage}</p>
                </div>
              </div>
            )}

            {/* Hash & Copy */}
            {transaction.hash && (
              <div className="space-y-3">
                <h3 className="text-sm font-semibold flex items-center gap-2">
                  Transaction Hash
                </h3>
                <div className="flex items-center gap-2">
                  <code className="flex-1 bg-muted px-3 py-2 rounded-md text-xs font-mono break-all border border-primary/5">
                    {transaction.hash}
                  </code>
                  <Button
                    variant="outline"
                    size="icon"
                    className="shrink-0 h-9 w-9"
                    onClick={handleCopyHash}
                  >
                    {copied ? <Check className="h-4 w-4 text-success" /> : <Copy className="h-4 w-4" />}
                  </Button>
                </div>
              </div>
            )}

            {/* Route Visualization */}
            <div className="space-y-3">
               <h3 className="text-sm font-semibold">Route Path</h3>
               <RouteVisualization 
                  path={transaction.routePath} 
                  className="bg-transparent border-primary/10 shadow-none p-0"
               />
            </div>

            {/* Explorer Links */}
            {transaction.hash && (
              <div className="space-y-3 pb-8">
                <h3 className="text-sm font-semibold">Block Explorers</h3>
                <div className="grid grid-cols-1 gap-2">
                  {explorers.map((explorer) => (
                    <a
                      key={explorer.name}
                      href={explorer.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center justify-between p-3 rounded-lg border border-primary/10 hover:bg-muted/50 hover:border-primary/20 transition-all group"
                    >
                      <span className="text-sm font-medium">{explorer.name}</span>
                      <ExternalLink className="h-4 w-4 text-muted-foreground group-hover:text-primary transition-colors" />
                    </a>
                  ))}
                </div>
              </div>
            )}
          </div>
        </ScrollArea>

        {/* Footer */}
        <div className="p-4 border-t border-primary/10 bg-muted/20">
          <Button className="w-full" onClick={() => onOpenChange(false)}>
            Close Details
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  );
}


"use client";
import { useCallback, useEffect, useRef, useState } from "react"
import { formatDistanceToNow } from "date-fns"
import { ArrowRight, Trash2, Download } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { ActivityTableSkeleton } from "@/components/shared/ActivityTableSkeleton"
import { CopyButton } from "@/components/shared/CopyButton"
import { ExplorerLink } from "@/components/shared/ExplorerLink"
import { useTransactionHistory } from "@/hooks/useTransactionHistory"
import { useVirtualWindow } from "@/hooks/useVirtualWindow"
import { TransactionRecord } from "@/types/transaction"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  ALL_EXPORT_COLUMNS,
  generateTransactionsCSV,
  triggerCSVDownload,
} from "@/lib/transaction-csv-export"

// Hardcode mock wallet to match DemoSwap
const MOCK_WALLET = "GBSU...XYZ9"
const ACTIVITY_VIRTUALIZATION_THRESHOLD = 24
const ACTIVITY_ROW_HEIGHT = 80

export function TransactionHistory({ onRetry }: { onRetry?: (tx: TransactionRecord) => void } = {}) {
  const { transactions, clearHistory } = useTransactionHistory(MOCK_WALLET)
  const [filterAsset, setFilterAsset] = useState<string>("ALL")
  const [sortKey, setSortKey] = useState<"date" | "amount">("date")
  const [isLoading, setIsLoading] = useState(true)
  const scrollRef = useRef<HTMLDivElement | null>(null)

  const [selectedColumns, setSelectedColumns] = useState<string[]>(() => {
    if (typeof window === "undefined") return ALL_EXPORT_COLUMNS.map((col) => col.key);
    try {
      const stored = localStorage.getItem("stellar_route_csv_export_columns");
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed) && parsed.length > 0) {
          return parsed;
        }
      }
    } catch (e) {
      console.error("Failed to load export columns", e);
    }
    return ALL_EXPORT_COLUMNS.map((col) => col.key);
  });
  const [isExporting, setIsExporting] = useState(false);
  const [exportProgress, setExportProgress] = useState(0);

  const filteredTxs = transactions.filter((tx) => {
    if (filterAsset === "ALL") return true
    return tx.fromAsset === filterAsset || tx.toAsset === filterAsset
  })

  const sortedTxs = [...filteredTxs].sort((a, b) => {
    if (sortKey === "date") {
      return b.timestamp - a.timestamp
    }
    return parseFloat(b.fromAmount) - parseFloat(a.fromAmount)
  })

  const handleColumnToggle = useCallback((key: string, checked: boolean) => {
    setSelectedColumns((prev) => {
      const next = checked ? [...prev, key] : prev.filter((k) => k !== key);
      localStorage.setItem("stellar_route_csv_export_columns", JSON.stringify(next));
      return next;
    });
  }, []);

  const handleExportCSV = useCallback(async () => {
    if (selectedColumns.length === 0) return;
    setIsExporting(true);
    setExportProgress(0);
    try {
      const csv = await generateTransactionsCSV(
        sortedTxs,
        selectedColumns,
        100,
        (progress) => setExportProgress(progress)
      );
      triggerCSVDownload(csv, `stellarroute_trade_activity_${Date.now()}.csv`);
    } catch (err) {
      console.error("Export failed", err);
    } finally {
      setIsExporting(false);
      setExportProgress(0);
    }
  }, [sortedTxs, selectedColumns]);

  useEffect(() => {
    const timer = setTimeout(() => {
      setIsLoading(false)
    }, 300)
    return () => clearTimeout(timer)
  }, [])

  const shouldVirtualize = sortedTxs.length > ACTIVITY_VIRTUALIZATION_THRESHOLD
  const virtualWindow = useVirtualWindow({
    containerRef: scrollRef,
    itemCount: sortedTxs.length,
    itemHeight: ACTIVITY_ROW_HEIGHT,
    overscan: 4,
    enabled: shouldVirtualize,
    defaultViewportHeight: ACTIVITY_ROW_HEIGHT * 6,
  })
  const visibleTxs = shouldVirtualize
    ? sortedTxs.slice(virtualWindow.startIndex, virtualWindow.endIndex)
    : sortedTxs

  const getStatusBadge = (status: TransactionRecord["status"]) => {
    switch (status) {
      case "confirmed":
        return <Badge className="bg-success" aria-label="Status: confirmed">Confirmed</Badge>
      case "failed":
        return <Badge variant="destructive" aria-label="Status: failed">Failed</Badge>
      case "pending":
        return <Badge variant="secondary" aria-label="Status: pending">Pending</Badge>
      case "submitted":
        return <Badge variant="secondary" aria-label="Status: submitted">Submitted</Badge>
      case "dropped":
        return <Badge variant="outline" aria-label="Status: dropped">Dropped</Badge>
      default:
        return <Badge variant="outline" aria-label={`Status: ${status}`}>{status}</Badge>
    }
  }

  return (
    <Card className="flex flex-col h-[calc(100vh-140px)] m-4 shadow-sm border-primary/10">
      <div className="p-4 border-b flex flex-col sm:flex-row justify-between items-center gap-4 bg-muted/30">
        <div>
          <h2 className="text-2xl font-bold">Transaction History</h2>
          <p className="text-sm text-muted-foreground mt-1 flex items-center gap-1">
            Wallet:{" "}
            <span className="font-mono text-foreground">{MOCK_WALLET}</span>
            <CopyButton value={MOCK_WALLET} label="Copy wallet address" />
          </p>
        </div>

        <div className="flex items-center gap-2">
          <select
            className="h-9 px-3 text-sm border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
            value={filterAsset}
            onChange={(e) => setFilterAsset(e.target.value)}
          >
            <option value="ALL">All Tokens</option>
            <option value="XLM">XLM</option>
            <option value="USDC">USDC</option>
          </select>

          <select
            className="h-9 px-3 text-sm border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
            value={sortKey}
            onChange={(e) => setSortKey(e.target.value as "date" | "amount")}
          >
            <option value="date">Sort by Date</option>
            <option value="amount">Sort by Amount</option>
          </select>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" className="flex items-center gap-2" disabled={isExporting} data-testid="csv-export-button">
                <Download className="h-4 w-4" />
                <span>{isExporting ? `Exporting (${Math.round(exportProgress * 100)}%)` : "Export"}</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-56" data-testid="csv-export-menu">
              <DropdownMenuLabel>Select Columns</DropdownMenuLabel>
              <DropdownMenuSeparator />
              {ALL_EXPORT_COLUMNS.map((col) => (
                <DropdownMenuCheckboxItem
                  key={col.key}
                  checked={selectedColumns.includes(col.key)}
                  onCheckedChange={(checked) => handleColumnToggle(col.key, checked)}
                  onSelect={(e) => e.preventDefault()}
                  data-testid={`column-checkbox-${col.key}`}
                >
                  {col.label}
                </DropdownMenuCheckboxItem>
              ))}
              <DropdownMenuSeparator />
              <DropdownMenuItem
                disabled={selectedColumns.length === 0 || isExporting}
                onClick={handleExportCSV}
                className="justify-center font-semibold text-primary focus:text-primary focus:bg-primary/10"
                data-testid="csv-download-button"
              >
                Download CSV
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>

          <Button variant="outline" size="icon" onClick={clearHistory} title="Clear History">
            <Trash2 className="h-4 w-4 text-destructive" />
          </Button>
        </div>
      </div>

      <div ref={scrollRef} data-testid="tx-history-scroll" className="flex-1 overflow-auto">
        {isLoading ? (
          <ActivityTableSkeleton />
        ) : sortedTxs.length === 0 ? (
          <div className="flex flex-col items-center justify-center p-12 text-center h-full">
            <div className="text-muted-foreground w-16 h-16 mb-4 opacity-50 bg-muted rounded-full flex items-center justify-center">
              <span className="text-2xl">📋</span>
            </div>
            <h3 className="text-xl font-semibold mb-1">No Transactions Found</h3>
            <p className="text-sm text-muted-foreground max-w-[250px]">
              You haven&apos;t made any swaps yet, or your filters are too restrictive.
            </p>
          </div>
        ) : (
          <div className="min-w-[720px]">
            <Table>
              <TableHeader className="bg-muted/50 sticky top-0 z-10">
                <TableRow>
                  <TableHead className="w-[180px]">Date</TableHead>
                  <TableHead>Swap</TableHead>
                  <TableHead>Rate</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="text-right">Explorer</TableHead>
                  <TableHead className="text-right">Retry</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {shouldVirtualize && virtualWindow.topSpacerHeight > 0 && (
                  <TableRow aria-hidden="true">
                    <TableCell
                      colSpan={6}
                      className="border-0 p-0"
                      style={{ height: virtualWindow.topSpacerHeight }}
                    />
                  </TableRow>
                )}
                {visibleTxs.map((tx) => (
                  <TableRow key={tx.id} data-testid={`tx-row-${tx.id}`}>
                    <TableCell className="font-medium">
                      <div className="flex flex-col">
                        <span>{new Date(tx.timestamp).toLocaleDateString()}</span>
                        <span className="text-xs text-muted-foreground whitespace-nowrap">
                          {formatDistanceToNow(tx.timestamp, { addSuffix: true })}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-3">
                        <div className="flex flex-col">
                          <span className="font-bold text-sm">-{tx.fromAmount}</span>
                          <span className="text-xs text-muted-foreground">{tx.fromAsset}</span>
                        </div>
                        <ArrowRight className="w-4 h-4 text-muted-foreground/50" />
                        <div className="flex flex-col">
                          <span className="font-bold text-sm text-success">+{tx.toAmount}</span>
                          <span className="text-xs text-muted-foreground">{tx.toAsset}</span>
                        </div>
                      </div>
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      1 {tx.fromAsset} = {tx.exchangeRate} {tx.toAsset}
                    </TableCell>
                    <TableCell>
                      {getStatusBadge(tx.status)}
                      {tx.errorMessage && (
                        <div
                          className="text-[10px] text-destructive mt-1 max-w-[150px] truncate"
                          title={tx.errorMessage}
                        >
                          {tx.errorMessage}
                        </div>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      {tx.hash ? (
                        <div className="inline-flex items-center gap-1 justify-end">
                          <CopyButton value={tx.hash} label="Copy transaction hash" />
                          <ExplorerLink
                            hash={tx.hash}
                            className="inline-flex items-center gap-1 text-xs text-primary hover:underline"
                          />
                        </div>
                      ) : (
                        <span className="text-xs text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      {(tx.status === "failed" || tx.status === "dropped") ? (
                        <button
                          className="text-xs text-primary hover:underline"
                          aria-label={`Retry ${tx.fromAsset}→${tx.toAsset} swap from ${new Date(tx.timestamp).toLocaleDateString()}`}
                          onClick={() => onRetry?.(tx)}
                        >
                          Retry
                        </button>
                      ) : (
                        <span />
                      )}
                    </TableCell>
                  </TableRow>
                ))}
                {shouldVirtualize && virtualWindow.bottomSpacerHeight > 0 && (
                  <TableRow aria-hidden="true">
                    <TableCell
                      colSpan={6}
                      className="border-0 p-0"
                      style={{ height: virtualWindow.bottomSpacerHeight }}
                    />
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        )}
      </div>
    </Card>
  )
}

import { TransactionRecord } from "@/types/transaction";

export interface CSVExportColumn {
  key: string;
  label: string;
}

export const ALL_EXPORT_COLUMNS: CSVExportColumn[] = [
  { key: "id", label: "ID" },
  { key: "date", label: "Date" },
  { key: "fromAmount", label: "From Amount" },
  { key: "fromAsset", label: "From Asset" },
  { key: "toAmount", label: "To Amount" },
  { key: "toAsset", label: "To Asset" },
  { key: "exchangeRate", label: "Exchange Rate" },
  { key: "priceImpact", label: "Price Impact" },
  { key: "minReceived", label: "Min Received" },
  { key: "networkFee", label: "Network Fee" },
  { key: "status", label: "Status" },
  { key: "hash", label: "Tx Hash" },
  { key: "errorMessage", label: "Error Message" },
  { key: "walletAddress", label: "Wallet Address" },
];

export function escapeCsvValue(val: unknown): string {
  if (val === undefined || val === null) return "";
  const str = String(val);
  if (str.includes(",") || str.includes('"') || str.includes("\n") || str.includes("\r")) {
    return `"${str.replace(/"/g, '""')}"`;
  }
  return str;
}

export function getColumnValueForTx(tx: TransactionRecord, key: string): string {
  switch (key) {
    case "id":
      return tx.id;
    case "date":
      return new Date(tx.timestamp).toISOString();
    case "fromAmount":
      return tx.fromAmount;
    case "fromAsset":
      return tx.fromAsset;
    case "toAmount":
      return tx.toAmount;
    case "toAsset":
      return tx.toAsset;
    case "exchangeRate":
      return tx.exchangeRate;
    case "priceImpact":
      return tx.priceImpact;
    case "minReceived":
      return tx.minReceived;
    case "networkFee":
      return tx.networkFee;
    case "status":
      return tx.status;
    case "hash":
      return tx.hash || "";
    case "errorMessage":
      return tx.errorMessage || "";
    case "walletAddress":
      return tx.walletAddress;
    default:
      return "";
  }
}

/**
 * Generates CSV content from transaction records in chunks asynchronously.
 * This prevents blocking the main thread during large exports.
 * The order of columns in the generated CSV will exactly match the order of keys in selectedKeys.
 */
export async function generateTransactionsCSV(
  transactions: TransactionRecord[],
  selectedKeys: string[],
  chunkSize = 100,
  onProgress?: (progress: number) => void
): Promise<string> {
  const columnMap = new Map(ALL_EXPORT_COLUMNS.map((col) => [col.key, col]));
  const activeColumns = selectedKeys
    .map((key) => columnMap.get(key))
    .filter((col): col is CSVExportColumn => col !== undefined);
  
  // Headers line
  const headers = activeColumns.map((col) => escapeCsvValue(col.label)).join(",");
  let csvContent = headers + "\n";
  
  if (transactions.length === 0) {
    return csvContent.trim() + "\n";
  }
  
  for (let i = 0; i < transactions.length; i += chunkSize) {
    const chunk = transactions.slice(i, i + chunkSize);
    const chunkRows = chunk.map((tx) => {
      return activeColumns
        .map((col) => escapeCsvValue(getColumnValueForTx(tx, col.key)))
        .join(",");
    });
    
    csvContent += chunkRows.join("\n") + "\n";
    
    if (onProgress) {
      onProgress(Math.min(1, (i + chunk.length) / transactions.length));
    }
    
    // Yield to the event loop if there are more chunks
    if (i + chunkSize < transactions.length) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  }
  
  return csvContent;
}

/**
 * Triggers the browser download of the CSV content.
 */
export function triggerCSVDownload(csvContent: string, filename = "stellarroute_trade_activity.csv") {
  const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.setAttribute("href", url);
  link.setAttribute("download", filename);
  link.style.visibility = "hidden";
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

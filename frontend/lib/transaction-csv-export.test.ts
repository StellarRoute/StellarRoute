import { describe, expect, it, vi } from "vitest";
import { TransactionRecord } from "@/types/transaction";
import {
  escapeCsvValue,
  getColumnValueForTx,
  generateTransactionsCSV,
} from "./transaction-csv-export";

const mockTransactions: TransactionRecord[] = [
  {
    id: "tx-1",
    timestamp: 1716724800000, // 2024-05-26T12:00:00.000Z
    fromAsset: "XLM",
    fromAmount: "100.5",
    toAsset: "USDC",
    toAmount: "12.3",
    exchangeRate: "0.122",
    priceImpact: "0.1",
    minReceived: "12.2",
    networkFee: "0.0001",
    routePath: [],
    status: "confirmed",
    hash: "hash-123",
    walletAddress: "GBSU...XYZ9",
  },
  {
    id: "tx-2",
    timestamp: 1716724860000, // 2024-05-26T12:01:00.000Z
    fromAsset: "USDC",
    fromAmount: "50",
    toAsset: "XLM",
    toAmount: "410",
    exchangeRate: "8.2",
    priceImpact: "0.2",
    minReceived: "408",
    networkFee: "0.0002",
    routePath: [],
    status: "failed",
    hash: "hash-456",
    errorMessage: "Transaction expired, slippage exceeded",
    walletAddress: "GBSU...XYZ9",
  },
];

describe("escapeCsvValue", () => {
  it("should return empty string for undefined and null", () => {
    expect(escapeCsvValue(undefined)).toBe("");
    expect(escapeCsvValue(null)).toBe("");
  });

  it("should return string representation of simple values", () => {
    expect(escapeCsvValue("hello")).toBe("hello");
    expect(escapeCsvValue(123)).toBe("123");
    expect(escapeCsvValue(true)).toBe("true");
  });

  it("should escape commas, double quotes and newlines", () => {
    expect(escapeCsvValue("hello, world")).toBe('"hello, world"');
    expect(escapeCsvValue('hello "world"')).toBe('"hello ""world"""');
    expect(escapeCsvValue("hello\nworld")).toBe('"hello\nworld"');
    expect(escapeCsvValue("hello\rworld")).toBe('"hello\rworld"');
  });
});

describe("getColumnValueForTx", () => {
  it("should map keys to transaction properties correctly", () => {
    const tx = mockTransactions[0];
    expect(getColumnValueForTx(tx, "id")).toBe("tx-1");
    expect(getColumnValueForTx(tx, "date")).toBe("2024-05-26T12:00:00.000Z");
    expect(getColumnValueForTx(tx, "fromAsset")).toBe("XLM");
    expect(getColumnValueForTx(tx, "fromAmount")).toBe("100.5");
    expect(getColumnValueForTx(tx, "toAsset")).toBe("USDC");
    expect(getColumnValueForTx(tx, "toAmount")).toBe("12.3");
    expect(getColumnValueForTx(tx, "exchangeRate")).toBe("0.122");
    expect(getColumnValueForTx(tx, "priceImpact")).toBe("0.1");
    expect(getColumnValueForTx(tx, "minReceived")).toBe("12.2");
    expect(getColumnValueForTx(tx, "networkFee")).toBe("0.0001");
    expect(getColumnValueForTx(tx, "status")).toBe("confirmed");
    expect(getColumnValueForTx(tx, "hash")).toBe("hash-123");
    expect(getColumnValueForTx(tx, "errorMessage")).toBe("");
    expect(getColumnValueForTx(tx, "walletAddress")).toBe("GBSU...XYZ9");
  });

  it("should handle error messages when present", () => {
    const tx = mockTransactions[1];
    expect(getColumnValueForTx(tx, "errorMessage")).toBe("Transaction expired, slippage exceeded");
  });
});

describe("generateTransactionsCSV", () => {
  it("should output CSV with headers matching selectedKeys in exact order (Acceptance Criteria: Tests validate CSV header order)", async () => {
    const selectedKeys = ["status", "fromAsset", "date", "errorMessage"];
    const csv = await generateTransactionsCSV(mockTransactions, selectedKeys);
    
    const lines = csv.trim().split("\n");
    expect(lines[0]).toBe("Status,From Asset,Date,Error Message");
    
    // Check first data line
    expect(lines[1]).toBe("confirmed,XLM,2024-05-26T12:00:00.000Z,");
    
    // Check second data line (which has an error message with a comma)
    expect(lines[2]).toBe('failed,USDC,2024-05-26T12:01:00.000Z,"Transaction expired, slippage exceeded"');
  });

  it("should respect different header orderings (Acceptance Criteria: Tests validate CSV header order)", async () => {
    const order1 = ["id", "hash", "status"];
    const csv1 = await generateTransactionsCSV(mockTransactions, order1);
    expect(csv1.split("\n")[0]).toBe("ID,Tx Hash,Status");

    const order2 = ["status", "id", "hash"];
    const csv2 = await generateTransactionsCSV(mockTransactions, order2);
    expect(csv2.split("\n")[0]).toBe("Status,ID,Tx Hash");
  });

  it("should return headers only with empty transactions", async () => {
    const selectedKeys = ["date", "fromAsset", "toAsset"];
    const csv = await generateTransactionsCSV([], selectedKeys);
    
    const lines = csv.trim().split("\n");
    expect(lines).toHaveLength(1);
    expect(lines[0]).toBe("Date,From Asset,To Asset");
  });

  it("should chunk exports and call onProgress callback correctly (Acceptance Criteria: Large exports streamed or chunked safely)", async () => {
    const selectedKeys = ["id", "status"];
    const onProgress = vi.fn();
    
    // Process with chunkSize = 1 (so we have 2 chunks)
    const csv = await generateTransactionsCSV(mockTransactions, selectedKeys, 1, onProgress);
    
    const lines = csv.trim().split("\n");
    expect(lines).toHaveLength(3);
    expect(lines[0]).toBe("ID,Status");
    
    // Progress should be called twice: once for 1/2 (0.5), once for 2/2 (1)
    expect(onProgress).toHaveBeenCalledTimes(2);
    expect(onProgress).toHaveBeenNthCalledWith(1, 0.5);
    expect(onProgress).toHaveBeenNthCalledWith(2, 1);
  });
});

import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/wallet/xdr-builder", () => ({
  XdrBuildError: class XdrBuildError extends Error {
    code: string;
    constructor(code: string, message: string) {
      super(message);
      this.code = code;
      this.name = "XdrBuildError";
    }
  },
}));

import {
  useTransactionLifecycle,
  defaultSignTransaction,
  defaultSubmitTransaction,
} from "../useTransactionLifecycle";

vi.mock("@/lib/notifications", () => ({
  dispatchTransactionNotification: vi.fn(),
  isNotificationSupported: vi.fn(() => true),
  buildNotificationTitle: vi.fn(),
  buildNotificationBody: vi.fn(),
  buildExplorerUrl: vi.fn(),
}));

const tradeParams = {
  fromAsset: "native",
  fromAmount: "10",
  toAsset: "USDC:GA5Z",
  toAmount: "9.95",
  exchangeRate: "0.995",
  priceImpact: "0.1",
  minReceived: "9.90 USDC",
  networkFee: "0.00001",
  routePath: [],
  walletAddress: "GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789",
};

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("useTransactionLifecycle — production signing guards (#735)", () => {
  it("fails fast when walletAddress is set but buildXdr is missing", async () => {
    const signFn = vi.fn().mockResolvedValue("signed_xdr");

    const { result } = renderHook(() =>
      useTransactionLifecycle({
        signTransaction: signFn,
        submitTransaction: vi.fn().mockResolvedValue({ hash: "hash" }),
      })
    );

    await act(async () => {
      await result.current.initiateSwap(tradeParams);
    });

    await waitFor(() => {
      expect(result.current.status).toBe("failed");
    });

    expect(result.current.errorMessage).toBe(
      "Transaction could not be built. Please refresh and try again."
    );
    expect(signFn).not.toHaveBeenCalled();
  });

  it("fails fast when walletAddress is set but default sign stub is used", async () => {
    const buildXdr = vi.fn().mockResolvedValue("AAAAunsigned_xdr");

    const { result } = renderHook(() =>
      useTransactionLifecycle({
        buildXdr,
        signTransaction: defaultSignTransaction,
        submitTransaction: vi.fn().mockResolvedValue({ hash: "hash" }),
      })
    );

    await act(async () => {
      await result.current.initiateSwap(tradeParams);
    });

    await waitFor(() => {
      expect(result.current.status).toBe("failed");
    });

    expect(result.current.errorMessage).toBe("Wallet not ready for signing.");
    expect(buildXdr).not.toHaveBeenCalled();
  });

  it("fails fast when walletAddress is set but default submit stub is used", async () => {
    const buildXdr = vi.fn().mockResolvedValue("AAAAunsigned_xdr");
    const signFn = vi.fn().mockResolvedValue("signed_xdr");

    const { result } = renderHook(() =>
      useTransactionLifecycle({
        buildXdr,
        signTransaction: signFn,
        submitTransaction: defaultSubmitTransaction,
      })
    );

    await act(async () => {
      await result.current.initiateSwap(tradeParams);
    });

    await waitFor(() => {
      expect(result.current.status).toBe("failed");
    });

    expect(result.current.errorMessage).toBe(
      "Transaction submission is not configured."
    );
    expect(buildXdr).not.toHaveBeenCalled();
    expect(signFn).not.toHaveBeenCalled();
  });

  it("maps wallet rejection to actionable user-facing message", async () => {
    const signFn = vi
      .fn()
      .mockRejectedValue(new Error("User declined transaction signing"));
    const buildXdr = vi.fn().mockResolvedValue("AAAAunsigned_xdr");

    const { result } = renderHook(() =>
      useTransactionLifecycle({
        buildXdr,
        signTransaction: signFn,
        submitTransaction: vi.fn().mockResolvedValue({ hash: "hash" }),
      })
    );

    await act(async () => {
      await result.current.initiateSwap(tradeParams);
    });

    await waitFor(() => {
      expect(result.current.status).toBe("failed");
    });

    expect(result.current.errorMessage).toBe(
      "Signature rejected. You can try again or dismiss."
    );
    expect(buildXdr).toHaveBeenCalled();
    expect(signFn).toHaveBeenCalledWith("AAAAunsigned_xdr");
  });

  it("still allows injected stubs when walletAddress is empty", async () => {
    const signFn = vi.fn().mockResolvedValue("signed_mock_xdr");
    const submitFn = vi.fn().mockResolvedValue({ hash: "stub_hash" });

    const { result } = renderHook(() =>
      useTransactionLifecycle({
        signTransaction: signFn,
        submitTransaction: submitFn,
      })
    );

    await act(async () => {
      await result.current.initiateSwap({ ...tradeParams, walletAddress: "" });
    });

    await waitFor(() => {
      expect(result.current.status).toBe("confirmed");
    });

    expect(signFn).toHaveBeenCalledWith("mock_xdr");
    expect(result.current.txHash).toBe("stub_hash");
  });
});

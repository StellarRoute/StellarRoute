import { describe, it, expect } from "vitest";
import {
  parseSlippageInput,
  isValidSlippage,
  getSlippageWarning,
  getSlippageWarningLevel,
  getSlippageWarningTier,
  getSlippageAcknowledgmentKey,
  requiresSlippageAcknowledgment,
} from "./slippage";

describe("Slippage Utils", () => {
  it("parses valid input", () => {
    expect(parseSlippageInput("0.5")).toBe(0.5);
    expect(parseSlippageInput("1")).toBe(1);
  });

  it("rejects invalid input", () => {
    expect(parseSlippageInput("")).toBeNull();
    expect(parseSlippageInput("abc")).toBeNull();
  });

  it("validates slippage bounds", () => {
    expect(isValidSlippage(0.5)).toBe(true);
    expect(isValidSlippage(50)).toBe(true);
    expect(isValidSlippage(0)).toBe(true);
    expect(isValidSlippage(-1)).toBe(false);
    expect(isValidSlippage(100)).toBe(false);
  });

  it("detects low slippage warning", () => {
    expect(getSlippageWarning(0.05)).toContain("Very low");
  });

  it("detects high slippage warning", () => {
    expect(getSlippageWarning(5)).toContain("High slippage");
  });

  it("detects normal slippage", () => {
    expect(getSlippageWarning(0.5)).toBeNull();
  });

  it("reports warning levels", () => {
    expect(getSlippageWarningLevel(0.05)).toBe("low");
    expect(getSlippageWarningLevel(5)).toBe("high");
    expect(getSlippageWarningLevel(0.5)).toBeNull();
  });

  it("assigns configurable slippage warning tiers", () => {
    expect(getSlippageWarningTier(0.05)).toBe("low");
    expect(getSlippageWarningTier(1)).toBe("elevated");
    expect(getSlippageWarningTier(4.99)).toBe("elevated");
    expect(getSlippageWarningTier(5)).toBe("high");
    expect(getSlippageWarningTier(0.5)).toBeNull();
  });

  it("requires explicit acknowledgment only for the high tier", () => {
    expect(requiresSlippageAcknowledgment(1)).toBe(false);
    expect(requiresSlippageAcknowledgment(5)).toBe(true);
  });

  it("changes acknowledgment key when amount, pair, or slippage changes", () => {
    const base = getSlippageAcknowledgmentKey({
      amount: "10",
      fromToken: "native",
      toToken: "USDC:GQUOTE",
      slippage: 5,
    });

    expect(
      getSlippageAcknowledgmentKey({
        amount: "11",
        fromToken: "native",
        toToken: "USDC:GQUOTE",
        slippage: 5,
      }),
    ).not.toBe(base);
    expect(
      getSlippageAcknowledgmentKey({
        amount: "10",
        fromToken: "USDC:GQUOTE",
        toToken: "native",
        slippage: 5,
      }),
    ).not.toBe(base);
    expect(
      getSlippageAcknowledgmentKey({
        amount: "10",
        fromToken: "native",
        toToken: "USDC:GQUOTE",
        slippage: 6,
      }),
    ).not.toBe(base);
  });
});

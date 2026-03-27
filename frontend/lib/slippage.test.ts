import { describe, it, expect } from "vitest";
import {
  parseSlippageInput,
  isValidSlippage,
  getSlippageWarning,
  getSlippageWarningLevel,
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
});

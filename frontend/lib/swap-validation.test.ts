import { describe, it, expect } from "vitest";
import { SwapValidationSchema, SWAP_VALIDATION_MESSAGES } from "./swap-validation";

const BASE = "native";
const COUNTER = "USDC:GA5Z...";

describe("SwapValidationSchema", () => {
  it("accepts valid inputs", () => {
    const result = SwapValidationSchema.validate({
      amount: "12.5",
      maxDecimals: 7,
      sellAssetId: BASE,
      buyAssetId: COUNTER,
      slippage: 0.5,
    });

    expect(result.isValid).toBe(true);
    expect(result.parsed.amount?.numeric).toBe(12.5);
    expect(result.fieldErrors.amount).toBeUndefined();
    expect(result.fieldErrors.pair).toBeUndefined();
    expect(result.fieldErrors.slippage).toBeUndefined();
  });

  it("requires amount on submit", () => {
    const result = SwapValidationSchema.validate(
      {
        amount: "",
        maxDecimals: 7,
        sellAssetId: BASE,
        buyAssetId: COUNTER,
        slippage: 0.5,
      },
      { mode: "submit" },
    );

    expect(result.isValid).toBe(false);
    expect(result.fieldErrors.amount).toBe(SWAP_VALIDATION_MESSAGES.amountRequired);
  });

  it("allows empty amount during input", () => {
    const result = SwapValidationSchema.validate(
      {
        amount: "",
        maxDecimals: 7,
        sellAssetId: BASE,
        buyAssetId: COUNTER,
        slippage: 0.5,
      },
      { mode: "input" },
    );

    expect(result.fieldErrors.amount).toBeUndefined();
  });

  it("flags invalid amounts", () => {
    const cases = ["abc", "1.2.3", "1e7", "-1", "0"];
    
    cases.forEach(amount => {
      const result = SwapValidationSchema.validate({
        amount,
        maxDecimals: 7,
        sellAssetId: BASE,
        buyAssetId: COUNTER,
        slippage: 0.5,
      });

      expect(result.isValid).toBe(false);
      expect(result.fieldErrors.amount).toBeDefined();
    });
  });

  it("flags precision-exceeded amounts", () => {
    const result = SwapValidationSchema.validate({
      amount: "1.123456789",
      maxDecimals: 7,
      sellAssetId: BASE,
      buyAssetId: COUNTER,
      slippage: 0.5,
    });

    expect(result.isValid).toBe(false);
    expect(result.fieldErrors.amount).toBe(SWAP_VALIDATION_MESSAGES.amountPrecision(7));
  });

  it("validates the asset pair", () => {
    const missing = SwapValidationSchema.validate({
      amount: "1",
      maxDecimals: 7,
      sellAssetId: "",
      buyAssetId: "",
      slippage: 0.5,
    });

    expect(missing.fieldErrors.pair).toBe(SWAP_VALIDATION_MESSAGES.pairRequired);

    const same = SwapValidationSchema.validate({
      amount: "1",
      maxDecimals: 7,
      sellAssetId: BASE,
      buyAssetId: BASE,
      slippage: 0.5,
    });

    expect(same.fieldErrors.pair).toBe(SWAP_VALIDATION_MESSAGES.pairSame);
  });

  it("validates slippage bounds and warnings", () => {
    const missing = SwapValidationSchema.validate(
      {
        amount: "1",
        maxDecimals: 7,
        sellAssetId: BASE,
        buyAssetId: COUNTER,
        slippage: null,
      },
      { mode: "submit" },
    );

    expect(missing.fieldErrors.slippage).toBe(
      SWAP_VALIDATION_MESSAGES.slippageRequired,
    );

    const outOfRange = SwapValidationSchema.validate({
      amount: "1",
      maxDecimals: 7,
      sellAssetId: BASE,
      buyAssetId: COUNTER,
      slippage: 51,
    });

    expect(outOfRange.fieldErrors.slippage).toBe(
      SWAP_VALIDATION_MESSAGES.slippageInvalid,
    );

    const negative = SwapValidationSchema.validate({
      amount: "1",
      maxDecimals: 7,
      sellAssetId: BASE,
      buyAssetId: COUNTER,
      slippage: -1,
    });

    expect(negative.fieldErrors.slippage).toBe(
      SWAP_VALIDATION_MESSAGES.slippageInvalid,
    );
  });

  it("handles edge cases for slippage", () => {
    // 0 is valid
    const zero = SwapValidationSchema.validate({
      amount: "1",
      maxDecimals: 7,
      sellAssetId: BASE,
      buyAssetId: COUNTER,
      slippage: 0,
    });
    expect(zero.isValid).toBe(true);

    // 50 is valid
    const fifty = SwapValidationSchema.validate({
      amount: "1",
      maxDecimals: 7,
      sellAssetId: BASE,
      buyAssetId: COUNTER,
      slippage: 50,
    });
    expect(fifty.isValid).toBe(true);
    
    // NaN should be handled
    const nan = SwapValidationSchema.validate({
      amount: "1",
      maxDecimals: 7,
      sellAssetId: BASE,
      buyAssetId: COUNTER,
      slippage: NaN,
    }, { mode: "submit" });
    expect(nan.fieldErrors.slippage).toBe(SWAP_VALIDATION_MESSAGES.slippageRequired);
  });

  it("can skip pair validation when configured", () => {
    const result = SwapValidationSchema.validate(
      {
        amount: "1",
        maxDecimals: 7,
        sellAssetId: "",
        buyAssetId: "",
        slippage: 0.5,
      },
      { requirePair: false },
    );

    expect(result.fieldErrors.pair).toBeUndefined();
  });

  it("supports future extension via additional input fields", () => {
    const result = SwapValidationSchema.validate({
      amount: "1",
      sellAssetId: BASE,
      buyAssetId: COUNTER,
      slippage: 0.5,
      newField: "some value"
    } as any);

    expect(result.isValid).toBe(true);
  });
});

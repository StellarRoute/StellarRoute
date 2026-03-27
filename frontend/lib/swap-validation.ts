import type { ParseSellAmountResult } from "./amount-input";
import {
  DEFAULT_ISSUED_MAX_DECIMALS,
  parseSellAmount,
} from "./amount-input";
import {
  getSlippageWarning,
  isValidSlippage,
  MAX_SLIPPAGE,
  MIN_SLIPPAGE,
} from "./slippage";

export type SwapValidationField = "amount" | "pair" | "slippage" | string;

export interface SwapValidationIssue {
  field: SwapValidationField;
  code: string;
  message: string;
}

export interface SwapValidationInput {
  amount: string;
  maxDecimals?: number;
  sellAssetId?: string | null;
  buyAssetId?: string | null;
  slippage: number | null;
  [key: string]: any; // Allow for future extension
}

export interface SwapValidationOptions {
  /**
   * In `input` mode, empty values are allowed without errors.
   * In `submit` mode, required fields must be present.
   */
  mode?: "input" | "submit";
  /**
   * When false, asset pair validation is skipped (useful for demo-only UIs).
   */
  requirePair?: boolean;
}

export interface SwapValidationResult {
  isValid: boolean;
  /**
   * Issues are additive so new fields can be introduced without
   * breaking existing consumers that only check known keys.
   */
  issues: SwapValidationIssue[];
  fieldErrors: Partial<Record<SwapValidationField, string>>;
  warnings: Partial<Record<SwapValidationField, string>>;
  amountResult: ParseSellAmountResult;
  parsed: {
    amount?: { normalized: string; numeric: number };
    slippage?: number;
    [key: string]: any;
  };
}

export const SWAP_VALIDATION_MESSAGES = {
  amountRequired: "Enter an amount.",
  amountInvalid: "Enter a valid amount.",
  amountPrecision: (max: number) => `Maximum ${max} decimal places for this asset.`,
  pairRequired: "Select tokens.",
  pairSame: "Select two different assets.",
  slippageRequired: "Enter a slippage value.",
  slippageInvalid: `Enter a slippage value between ${MIN_SLIPPAGE}% and ${MAX_SLIPPAGE}%.`,
} as const;

function resolveMaxDecimals(maxDecimals?: number): number {
  if (
    typeof maxDecimals === "number" &&
    Number.isInteger(maxDecimals) &&
    maxDecimals >= 0
  ) {
    return maxDecimals;
  }
  return DEFAULT_ISSUED_MAX_DECIMALS;
}

/**
 * Shared validation schema for swap inputs.
 * Centralizes validation logic for amount, asset pair, and slippage.
 */
export const SwapValidationSchema = {
  validate: (
    input: SwapValidationInput,
    options: SwapValidationOptions = {},
  ): SwapValidationResult => {
    const mode = options.mode ?? "submit";
    const requirePair = options.requirePair ?? true;

    const issues: SwapValidationIssue[] = [];
    const fieldErrors: Partial<Record<SwapValidationField, string>> = {};
    const warnings: Partial<Record<SwapValidationField, string>> = {};
    const parsed: SwapValidationResult["parsed"] = {};

    const addIssue = (issue: SwapValidationIssue) => {
      issues.push(issue);
      if (!fieldErrors[issue.field]) {
        fieldErrors[issue.field] = issue.message;
      }
    };

    // 1. Pair Validation
    if (requirePair) {
      const sellAssetId = input.sellAssetId ?? "";
      const buyAssetId = input.buyAssetId ?? "";
      if (!sellAssetId || !buyAssetId) {
        addIssue({
          field: "pair",
          code: "pair_missing",
          message: SWAP_VALIDATION_MESSAGES.pairRequired,
        });
      } else if (sellAssetId === buyAssetId) {
        addIssue({
          field: "pair",
          code: "pair_same",
          message: SWAP_VALIDATION_MESSAGES.pairSame,
        });
      }
    }

    // 2. Amount Validation
    const maxDecimals = resolveMaxDecimals(input.maxDecimals);
    const amountResult = parseSellAmount(input.amount ?? "", maxDecimals);

    if (amountResult.status === "empty") {
      if (mode === "submit") {
        addIssue({
          field: "amount",
          code: "amount_empty",
          message: SWAP_VALIDATION_MESSAGES.amountRequired,
        });
      }
    } else if (amountResult.status === "invalid") {
      addIssue({
        field: "amount",
        code: "amount_invalid",
        message: amountResult.message || SWAP_VALIDATION_MESSAGES.amountInvalid,
      });
    } else if (amountResult.status === "precision_exceeded") {
      addIssue({
        field: "amount",
        code: "amount_precision",
        message: amountResult.message || SWAP_VALIDATION_MESSAGES.amountPrecision(maxDecimals),
      });
    } else if (amountResult.status === "ok") {
      parsed.amount = {
        normalized: amountResult.normalized,
        numeric: amountResult.numeric,
      };
    }

    // 3. Slippage Validation
    if (input.slippage === null || typeof input.slippage === "undefined" || Number.isNaN(input.slippage)) {
      if (mode === "submit") {
        addIssue({
          field: "slippage",
          code: "slippage_missing",
          message: SWAP_VALIDATION_MESSAGES.slippageRequired,
        });
      }
    } else if (!isValidSlippage(input.slippage)) {
      addIssue({
        field: "slippage",
        code: "slippage_out_of_range",
        message: SWAP_VALIDATION_MESSAGES.slippageInvalid,
      });
    } else {
      parsed.slippage = input.slippage;
      const warning = getSlippageWarning(input.slippage);
      if (warning) {
        warnings.slippage = warning;
      }
    }

    return {
      isValid: issues.length === 0,
      issues,
      fieldErrors,
      warnings,
      amountResult,
      parsed,
    };
  },
};

/**
 * Legacy wrapper for the schema-based validation.
 * @deprecated Use SwapValidationSchema.validate instead.
 */
export function validateSwapInputs(
  input: SwapValidationInput,
  options: SwapValidationOptions = {},
): SwapValidationResult {
  return SwapValidationSchema.validate(input, options);
}

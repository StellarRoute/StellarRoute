import { describe, expect, it } from "vitest";

import {
  createSwapTranslator,
  resolveSwapLocale,
  SWAP_FALLBACK_LOCALE,
} from "@/lib/swap-i18n";

describe("swap i18n", () => {
  it("uses zh-CN translations when they are available", () => {
    const { locale, t } = createSwapTranslator("zh-CN");

    expect(locale).toBe("zh-CN");
    expect(t("swap.card.title")).toBe("兑换");
    expect(t("swap.pair.balance", { amount: "1,000" })).toBe("余额：1,000");
  });

  it("falls back to en-US for unsupported swap locales", () => {
    const translator = createSwapTranslator("fr-FR");

    expect(resolveSwapLocale("fr-FR")).toBe("en-US");
    expect(translator.locale).toBe("en-US");
    expect(translator.fallbackLocale).toBe(SWAP_FALLBACK_LOCALE);
    expect(translator.t("swap.card.title")).toBe("Swap");
  });
});

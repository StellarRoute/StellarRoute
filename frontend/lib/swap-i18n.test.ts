import { describe, expect, it, vi } from "vitest";

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

  it("uses es-ES translations when they are available", () => {
    const { locale, t } = createSwapTranslator("es-ES");

    expect(locale).toBe("es-ES");
    expect(t("swap.card.title")).toBe("Intercambiar");
    expect(t("swap.pair.balance", { amount: "1,000" })).toBe("Saldo: 1,000");
  });

  it("uses de-DE translations when they are available", () => {
    const { locale, t } = createSwapTranslator("de-DE");

    expect(locale).toBe("de-DE");
    expect(t("swap.card.title")).toBe("Tauschen");
  });

  it("uses fr-FR translations when they are available", () => {
    const { locale, t } = createSwapTranslator("fr-FR");

    expect(locale).toBe("fr-FR");
    expect(t("swap.card.title")).toBe("Échanger");
  });

  it("uses ja-JP translations when they are available", () => {
    const { locale, t } = createSwapTranslator("ja-JP");

    expect(locale).toBe("ja-JP");
    expect(t("swap.card.title")).toBe("スワップ");
  });

  it("falls back to en-US for unsupported swap locales", () => {
    const translator = createSwapTranslator("pt-BR" as any);

    expect(resolveSwapLocale("pt-BR" as any)).toBe("en-US");
    expect(translator.locale).toBe("en-US");
    expect(translator.fallbackLocale).toBe(SWAP_FALLBACK_LOCALE);
    expect(translator.t("swap.card.title")).toBe("Swap");
  });

  it("warns in development when falling back for a missing key", () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const translator = createSwapTranslator("de-DE");

    const value = translator.t("swap.nonexistent.key" as never);
    expect(value).toBe("swap.nonexistent.key");
    expect(warnSpy).toHaveBeenCalled();
    warnSpy.mockRestore();
  });
});

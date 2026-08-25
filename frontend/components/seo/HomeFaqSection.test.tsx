import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { HomeFaqSection } from "./HomeFaqSection";
import { HOME_FAQS } from "@/lib/seo";

describe("HomeFaqSection", () => {
  it("renders every FAQ question and answer", () => {
    render(<HomeFaqSection />);

    expect(
      screen.getByRole("heading", {
        name: "Stellar DEX and cross-chain swap questions",
      }),
    ).toBeInTheDocument();

    for (const faq of HOME_FAQS) {
      expect(screen.getByText(faq.question)).toBeInTheDocument();
      expect(screen.getByText(faq.answer)).toBeInTheDocument();
    }

    expect(
      screen.getByRole("link", { name: "how Stellar DEX aggregation works" }),
    ).toHaveAttribute("href", "/stellar-dex-aggregator");
    expect(
      screen.getByRole("link", { name: "Stellar cross-chain swaps" }),
    ).toHaveAttribute("href", "/cross-chain-swap");
  });
});

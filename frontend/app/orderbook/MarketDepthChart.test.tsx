import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { MarketDepthChart } from "./MarketDepthChart";

const bids = [
  { price: 1.2, amount: 100, total: 0 },
  { price: 1.19, amount: 200, total: 0 },
];
const asks = [
  { price: 1.21, amount: 150, total: 0 },
  { price: 1.22, amount: 50, total: 0 },
];

describe("MarketDepthChart", () => {
  afterEach(cleanup);

  it("exposes an accessible name on the depth chart svg", () => {
    render(<MarketDepthChart bids={bids} asks={asks} />);

    const chart = screen.getByRole("img", { name: /market depth chart/i });
    expect(chart).toBeInTheDocument();
    expect(chart).toHaveAttribute("aria-label", expect.stringContaining("300"));
    expect(chart).toHaveAttribute("aria-label", expect.stringContaining("200"));
  });

  it("describes bid and ask depth via title elements on each fill area", () => {
    const { container } = render(<MarketDepthChart bids={bids} asks={asks} />);

    const titles = Array.from(container.querySelectorAll("polygon > title")).map(
      (el) => el.textContent,
    );
    expect(titles.some((t) => /bid depth/i.test(t ?? ""))).toBe(true);
    expect(titles.some((t) => /ask depth/i.test(t ?? ""))).toBe(true);
  });
});

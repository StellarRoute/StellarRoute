import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";

import { RouteVisualization } from "@/components/shared/RouteVisualization";
import type { PathStep } from "@/types";

const path: PathStep[] = [
  {
    from_asset: {
      asset_type: "native",
    },
    to_asset: {
      asset_type: "credit_alphanum4",
      asset_code: "USDC",
      asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    },
    price: "0.98",
    source: "sdex",
  },
];

describe("RouteVisualization", () => {
  it("renders route nodes with stable asset fallbacks", () => {
    render(<RouteVisualization path={path} />);

    expect(
      screen.getByRole("group", { name: "Start asset: XLM" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("group", { name: "Destination asset: USDC" })
    ).toBeInTheDocument();
    expect(screen.getAllByText("USD").length).toBeGreaterThan(0);
  });
});

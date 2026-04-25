import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { AssetIcon } from "@/components/shared/AssetIcon";

describe("AssetIcon", () => {
  it("renders a stable symbol fallback when no icon source is provided", () => {
    render(<AssetIcon symbol="USDC" />);

    expect(screen.getByText("US")).toBeInTheDocument();
    expect(screen.queryByRole("img")).toBeNull();
  });

  it("falls back to initials when the icon fails to load", () => {
    render(<AssetIcon symbol="AQUA" src="https://cdn.example.com/aqua.png" />);

    const image = screen.getByRole("img", {
      name: "AQUA icon",
      hidden: true,
    });
    fireEvent.error(image);

    expect(screen.getByText("AQ")).toBeInTheDocument();
    expect(
      screen.queryByRole("img", { name: "AQUA icon", hidden: true })
    ).toBeNull();
  });
});

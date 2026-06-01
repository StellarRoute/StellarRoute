/**
 * Tests for the font scale accessibility control (issue #522).
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FontScaleControl } from "./FontScaleControl";
import { SettingsProvider } from "@/components/providers/settings-provider";

vi.mock("next-themes", () => ({
  useTheme: () => ({ theme: "dark", setTheme: vi.fn() }),
}));

function Wrapper({ children }: { children: React.ReactNode }) {
  return <SettingsProvider>{children}</SettingsProvider>;
}

describe("FontScaleControl", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.style.removeProperty("font-size");
  });

  it("renders preset buttons for all 5 scale steps", () => {
    render(
      <Wrapper>
        <FontScaleControl />
      </Wrapper>
    );

    expect(screen.getByRole("radio", { name: /100%/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /125%/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /150%/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /175%/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /200%/i })).toBeInTheDocument();
  });

  it("defaults to 100% (scale = 1.0)", () => {
    render(
      <Wrapper>
        <FontScaleControl />
      </Wrapper>
    );
    expect(screen.getByRole("radio", { name: /100%/i })).toHaveAttribute(
      "aria-checked",
      "true"
    );
  });

  it("updates selection when a preset button is clicked", async () => {
    const user = userEvent.setup();
    render(
      <Wrapper>
        <FontScaleControl />
      </Wrapper>
    );

    await user.click(screen.getByRole("radio", { name: /150%/i }));

    expect(screen.getByRole("radio", { name: /150%/i })).toHaveAttribute(
      "aria-checked",
      "true"
    );
    expect(screen.getByRole("radio", { name: /100%/i })).toHaveAttribute(
      "aria-checked",
      "false"
    );
  });

  it("applies the font-size to <html> when a scale is chosen", async () => {
    const user = userEvent.setup();
    render(
      <Wrapper>
        <FontScaleControl />
      </Wrapper>
    );

    // 1.5 × 16 px = 24 px
    await user.click(screen.getByRole("radio", { name: /150%/i }));
    expect(document.documentElement.style.getPropertyValue("font-size")).toBe("24px");
  });

  it("applies 200% (32 px) at max scale", async () => {
    const user = userEvent.setup();
    render(
      <Wrapper>
        <FontScaleControl />
      </Wrapper>
    );

    await user.click(screen.getByRole("radio", { name: /200%/i }));
    expect(document.documentElement.style.getPropertyValue("font-size")).toBe("32px");
  });

  it("persists the font scale in localStorage", async () => {
    const user = userEvent.setup();
    render(
      <Wrapper>
        <FontScaleControl />
      </Wrapper>
    );

    await user.click(screen.getByRole("radio", { name: /175%/i }));

    const stored = JSON.parse(
      localStorage.getItem("stellar_route_settings") ?? "{}"
    );
    expect(stored.fontScale).toBe(1.75);
  });

  it("renders a live preview section", () => {
    render(
      <Wrapper>
        <FontScaleControl />
      </Wrapper>
    );
    expect(screen.getByLabelText(/font scale preview/i)).toBeInTheDocument();
  });

  it("renders a range slider", () => {
    render(
      <Wrapper>
        <FontScaleControl />
      </Wrapper>
    );
    expect(screen.getByRole("slider")).toBeInTheDocument();
  });
});

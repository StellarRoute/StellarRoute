/**
 * Tests for the accent colour picker (issue #521).
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AccentColorPicker } from "./AccentColorPicker";
import { SettingsProvider } from "@/components/providers/settings-provider";

// next-themes is not needed in unit tests
vi.mock("next-themes", () => ({
  useTheme: () => ({ theme: "dark", setTheme: vi.fn() }),
}));

function Wrapper({ children }: { children: React.ReactNode }) {
  return <SettingsProvider>{children}</SettingsProvider>;
}

describe("AccentColorPicker", () => {
  beforeEach(() => {
    localStorage.clear();
    // reset CSS custom properties between tests
    document.documentElement.style.removeProperty("--primary");
  });

  it("renders a colour swatch for every preset", () => {
    render(
      <Wrapper>
        <AccentColorPicker />
      </Wrapper>
    );

    // The presets are indigo, sky, emerald, rose, amber, violet
    const radios = screen.getAllByRole("radio");
    expect(radios.length).toBeGreaterThanOrEqual(6);
  });

  it("marks the default (indigo) as checked initially", () => {
    render(
      <Wrapper>
        <AccentColorPicker />
      </Wrapper>
    );
    expect(screen.getByRole("radio", { name: /indigo/i })).toHaveAttribute(
      "aria-checked",
      "true"
    );
  });

  it("updates to the selected colour when clicked", async () => {
    const user = userEvent.setup();
    render(
      <Wrapper>
        <AccentColorPicker />
      </Wrapper>
    );

    await user.click(screen.getByRole("radio", { name: /rose/i }));

    expect(screen.getByRole("radio", { name: /rose/i })).toHaveAttribute(
      "aria-checked",
      "true"
    );
    expect(screen.getByRole("radio", { name: /indigo/i })).toHaveAttribute(
      "aria-checked",
      "false"
    );
  });

  it("applies the CSS --primary variable when a colour is chosen", async () => {
    const user = userEvent.setup();
    render(
      <Wrapper>
        <AccentColorPicker />
      </Wrapper>
    );

    await user.click(screen.getByRole("radio", { name: /emerald/i }));

    expect(document.documentElement.style.getPropertyValue("--primary")).toBe(
      "#10b981"
    );
  });

  it("persists the selection in localStorage", async () => {
    const user = userEvent.setup();
    render(
      <Wrapper>
        <AccentColorPicker />
      </Wrapper>
    );

    await user.click(screen.getByRole("radio", { name: /violet/i }));

    const stored = JSON.parse(
      localStorage.getItem("stellar_route_settings") ?? "{}"
    );
    expect(stored.accentColor).toBe("violet");
  });

  it("renders a custom colour input", () => {
    render(
      <Wrapper>
        <AccentColorPicker />
      </Wrapper>
    );
    expect(screen.getByLabelText(/pick a custom accent colour/i)).toBeInTheDocument();
  });
});

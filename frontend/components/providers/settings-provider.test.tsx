import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { SettingsProvider, useSettings } from "@/components/providers/settings-provider";

function TestConsumer() {
  const { settings, updateSlippage, updateHighContrast } = useSettings();

  return (
    <>
      <div data-testid="slippage">{settings.slippageTolerance}</div>
      <div data-testid="high-contrast">{String(settings.highContrast)}</div>
      <button onClick={() => updateSlippage(2)}>Set 2%</button>
      <button onClick={() => updateSlippage(100)}>Set 100%</button>
      <button onClick={() => updateHighContrast(true)}>Enable HC</button>
      <button onClick={() => updateHighContrast(false)}>Disable HC</button>
    </>
  );
}

describe("SettingsProvider", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.classList.remove("high-contrast");
  });

  it("initializes to default settings and persists updates", async () => {
    render(
      <SettingsProvider>
        <TestConsumer />
      </SettingsProvider>,
    );

    expect(screen.getByTestId("slippage").textContent).toBe("0.5");

    await userEvent.click(screen.getByText("Set 2%"));
    expect(screen.getByTestId("slippage").textContent).toBe("2");

    const stored = JSON.parse(window.localStorage.getItem("stellar_route_settings") ?? "{}");
    expect(stored.slippageTolerance).toBe(2);
  });

  it("prevents invalid slippage values outside 0-50", async () => {
    render(
      <SettingsProvider>
        <TestConsumer />
      </SettingsProvider>,
    );

    const set2Button = screen.getAllByRole("button", { name: "Set 2%" })[0];
    const set100Button = screen.getAllByRole("button", { name: "Set 100%" })[0];

    await userEvent.click(set2Button);
    expect(screen.getAllByTestId("slippage")[0].textContent).toBe("2");

    await userEvent.click(set100Button);
    expect(screen.getAllByTestId("slippage")[0].textContent).toBe("2");

    const stored = JSON.parse(window.localStorage.getItem("stellar_route_settings") ?? "{}");
    expect(stored.slippageTolerance).toBe(2);
  });

  it("defaults highContrast to false", () => {
    render(
      <SettingsProvider>
        <TestConsumer />
      </SettingsProvider>,
    );

    expect(screen.getByTestId("high-contrast").textContent).toBe("false");
    expect(document.documentElement.classList.contains("high-contrast")).toBe(false);
  });

  it("adds high-contrast class to <html> when enabled", async () => {
    render(
      <SettingsProvider>
        <TestConsumer />
      </SettingsProvider>,
    );

    await userEvent.click(screen.getByText("Enable HC"));

    expect(screen.getByTestId("high-contrast").textContent).toBe("true");
    expect(document.documentElement.classList.contains("high-contrast")).toBe(true);
  });

  it("removes high-contrast class from <html> when disabled", async () => {
    render(
      <SettingsProvider>
        <TestConsumer />
      </SettingsProvider>,
    );

    await userEvent.click(screen.getByText("Enable HC"));
    expect(document.documentElement.classList.contains("high-contrast")).toBe(true);

    await userEvent.click(screen.getByText("Disable HC"));
    expect(document.documentElement.classList.contains("high-contrast")).toBe(false);
  });

  it("persists highContrast to localStorage", async () => {
    render(
      <SettingsProvider>
        <TestConsumer />
      </SettingsProvider>,
    );

    await userEvent.click(screen.getByText("Enable HC"));

    const stored = JSON.parse(window.localStorage.getItem("stellar_route_settings") ?? "{}");
    expect(stored.highContrast).toBe(true);
  });

  it("restores highContrast from localStorage on mount", () => {
    window.localStorage.setItem(
      "stellar_route_settings",
      JSON.stringify({ highContrast: true, slippageTolerance: 0.5 }),
    );

    render(
      <SettingsProvider>
        <TestConsumer />
      </SettingsProvider>,
    );

    expect(screen.getByTestId("high-contrast").textContent).toBe("true");
    expect(document.documentElement.classList.contains("high-contrast")).toBe(true);
  });
});

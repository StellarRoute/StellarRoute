import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { SettingsProvider, useSettings } from "@/components/providers/settings-provider";

function TestConsumer() {
  const { settings, updateSlippage } = useSettings();

  return (
    <>
      <div data-testid="slippage">{settings.slippageTolerance}</div>
      <button onClick={() => updateSlippage(2)}>Set 2%</button>
      <button onClick={() => updateSlippage(100)}>Set 100%</button>
    </>
  );
}

describe("SettingsProvider", () => {
  it("initializes to default settings and persists updates", async () => {
    window.localStorage.clear();

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
    window.localStorage.clear();

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
});

import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SlippageSettings } from "./SlippageSettings";
import { SettingsProvider } from "@/components/providers/settings-provider";

describe("SlippageSettings", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });
  
  afterEach(() => {
    cleanup();
  });

  const renderComponent = () => {
    return render(
      <SettingsProvider>
        <SlippageSettings />
      </SettingsProvider>
    );
  };

  it("renders correctly with default values", () => {
    renderComponent();
    expect(screen.getByText("Slippage Tolerance")).toBeTruthy();
    // Default is 0.5%
    expect(screen.getByText("0.5%")).toBeTruthy();
  });

  it("changes value when a preset button is clicked", async () => {
    renderComponent();
    
    // Click 1.0% preset
    const presetBtn = screen.getByText("Aggressive");
    await userEvent.click(presetBtn);

    expect(screen.getByText("1%")).toBeTruthy();
  });

  it("handles custom input value", async () => {
    renderComponent();
    const input = screen.getByPlaceholderText("Custom");
    
    await userEvent.type(input, "2.5");
    expect(screen.getByText("2.5%")).toBeTruthy();
  });

  it("shows warning for low slippage (< 0.1%)", async () => {
    renderComponent();
    const input = screen.getByPlaceholderText("Custom");
    
    await userEvent.clear(input);
    await userEvent.type(input, "0.05");
    
    expect(screen.getByText(/may fail if the price moves/i)).toBeTruthy();
  });

  it("shows warning for high slippage (> 5%)", async () => {
    renderComponent();
    const input = screen.getByPlaceholderText("Custom");
    
    await userEvent.clear(input);
    await userEvent.type(input, "10");
    
    expect(screen.getByText(/increases the risk of frontrunning/i)).toBeTruthy();
  });
});

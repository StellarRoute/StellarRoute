import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi, afterEach } from "vitest";
import { SlippageSettings } from "./SlippageSettings";

describe("SlippageSettings", () => {
  afterEach(() => cleanup());

  it("calls onChange when a preset button is clicked", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SlippageSettings value={0.5} onChange={onChange} />);

    // Match "1%" instead of "1.0%"
    const preset1 = screen.getByRole("button", { name: /^1%$/i });
    await user.click(preset1);
    expect(onChange).toHaveBeenCalledWith(1.0);
  });

  it("calls onChange with custom input value", async () => {
    const onChange = vi.fn();
    render(<SlippageSettings value={0.5} onChange={onChange} />);

    const input = screen.getByPlaceholderText(/custom/i);
    fireEvent.change(input, { target: { value: "2.5" } });
    expect(onChange).toHaveBeenCalledWith(2.5);
  });

  it("clamps custom input between 0.01 and 50", async () => {
    const onChange = vi.fn();
    render(<SlippageSettings value={0.5} onChange={onChange} />);

    const input = screen.getByPlaceholderText(/custom/i);
    fireEvent.change(input, { target: { value: "60" } });
    expect(onChange).toHaveBeenCalledWith(50);
  });

  it("shows warning for low slippage (< 0.1%)", () => {
    render(<SlippageSettings value={0.05} onChange={() => {}} />);
    expect(screen.getByText(/transaction may fail/i)).toBeInTheDocument();
  });

  it("shows warning for high slippage (> 5%)", () => {
    render(<SlippageSettings value={6.0} onChange={() => {}} />);
    expect(screen.getByText(/increases the risk of frontrunning/i)).toBeInTheDocument();
  });
});

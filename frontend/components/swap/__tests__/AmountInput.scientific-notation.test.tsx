/**
 * W12-FE-13 — Swap amount input scientific notation guard.
 *
 * Values like "1e5", "2E10", "1.5e-3" must never reach the onChange callback
 * and must never display in the input field.
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { AmountInput } from "../AmountInput";

describe("AmountInput – scientific notation guard (W12-FE-13)", () => {
  it("rejects 'e' notation typed directly (1e5)", () => {
    const onChange = vi.fn();
    render(<AmountInput value="" onChange={onChange} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;

    fireEvent.change(input, { target: { value: "1e5" } });

    // Input must not display the scientific notation string.
    expect(input.value).not.toBe("1e5");
    // onChange must not have been called with the bad value.
    expect(onChange).not.toHaveBeenCalledWith("1e5");
  });

  it("rejects uppercase 'E' notation (2E10)", () => {
    const onChange = vi.fn();
    render(<AmountInput value="" onChange={onChange} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;

    fireEvent.change(input, { target: { value: "2E10" } });

    expect(input.value).not.toBe("2E10");
    expect(onChange).not.toHaveBeenCalledWith("2E10");
  });

  it("rejects negative exponent notation (1.5e-3)", () => {
    const onChange = vi.fn();
    render(<AmountInput value="" onChange={onChange} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;

    fireEvent.change(input, { target: { value: "1.5e-3" } });

    expect(input.value).not.toBe("1.5e-3");
    expect(onChange).not.toHaveBeenCalledWith("1.5e-3");
  });

  it("rejects '+' exponent notation (1e+5)", () => {
    const onChange = vi.fn();
    render(<AmountInput value="" onChange={onChange} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;

    fireEvent.change(input, { target: { value: "1e+5" } });

    expect(input.value).not.toBe("1e+5");
    expect(onChange).not.toHaveBeenCalledWith("1e+5");
  });

  it("rejects bare 'e' without digits", () => {
    const onChange = vi.fn();
    render(<AmountInput value="" onChange={onChange} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;

    fireEvent.change(input, { target: { value: "e" } });

    expect(input.value).not.toBe("e");
    expect(onChange).not.toHaveBeenCalledWith("e");
  });

  it("still accepts valid decimal strings after a rejected notation attempt", () => {
    const onChange = vi.fn();
    render(<AmountInput value="" onChange={onChange} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;

    // First reject sci notation.
    fireEvent.change(input, { target: { value: "1e5" } });
    // Then type a valid amount.
    fireEvent.change(input, { target: { value: "100000" } });

    expect(onChange).toHaveBeenCalledWith("100000");
    expect(input.value).toBe("100000");
  });

  it("does not suppress an error alert for precision violations (unrelated to sci notation)", () => {
    render(<AmountInput value="" decimals={2} onChange={vi.fn()} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;

    fireEvent.change(input, { target: { value: "1.999" } });

    // Precision guard still fires — sci notation guard must not interfere.
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
});

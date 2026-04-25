import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { HighImpactConfirmModal } from "./HighImpactConfirmModal";

describe("HighImpactConfirmModal", () => {
  const defaultProps = {
    isOpen: true,
    onClose: vi.fn(),
    onConfirm: vi.fn(),
    priceImpact: 5.5,
    fromAmount: "100",
    fromSymbol: "XLM",
    toAmount: "94.5",
    toSymbol: "USDC",
  };

  it("renders correctly with trade summary", () => {
    render(<HighImpactConfirmModal {...defaultProps} />);
    expect(screen.getByText("High Price Impact")).toBeInTheDocument();
    expect(screen.getByText("100 XLM")).toBeInTheDocument();
    expect(screen.getByText("94.5000 USDC")).toBeInTheDocument();
    expect(screen.getByText("5.50%")).toBeInTheDocument();
  });

  it("disables confirm button until checkbox is checked", () => {
    render(<HighImpactConfirmModal {...defaultProps} />);
    const confirmButton = screen.getByRole("button", { name: /confirm swap/i });
    expect(confirmButton).toBeDisabled();

    const checkbox = screen.getByRole("checkbox");
    fireEvent.click(checkbox);
    expect(confirmButton).toBeEnabled();
  });

  it("calls onConfirm and onClose when confirmed", async () => {
    const user = userEvent.setup();
    render(<HighImpactConfirmModal {...defaultProps} />);
    
    // Check risk understanding checkbox
    const checkbox = screen.getByRole("checkbox");
    await user.click(checkbox);

    // Click confirm
    const confirmButton = screen.getByRole("button", { name: /confirm swap/i });
    await user.click(confirmButton);

    expect(defaultProps.onConfirm).toHaveBeenCalled();
    expect(defaultProps.onClose).toHaveBeenCalled();
  });

  it("calls onClose when cancel is clicked", async () => {
    const user = userEvent.setup();
    render(<HighImpactConfirmModal {...defaultProps} />);
    
    const cancelButton = screen.getByRole("button", { name: /cancel/i });
    await user.click(cancelButton);

    expect(defaultProps.onClose).toHaveBeenCalled();
  });
});

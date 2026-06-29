import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { SlippageRiskNotice } from "./SlippageRiskNotice";

describe("SlippageRiskNotice", () => {
  afterEach(() => cleanup());

  it("does not render for normal slippage", () => {
    const { container } = render(
      <SlippageRiskNotice
        slippage={0.5}
        acknowledged={false}
        onAcknowledgedChange={() => {}}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("renders elevated tier copy without requiring acknowledgment", () => {
    render(
      <SlippageRiskNotice
        slippage={2}
        acknowledged={false}
        onAcknowledgedChange={() => {}}
      />,
    );

    expect(screen.getByText(/Elevated slippage tolerance/i)).toBeInTheDocument();
    expect(screen.queryByLabelText(/Acknowledge high slippage risk/i)).not.toBeInTheDocument();
  });

  it("requires explicit acknowledgment for high slippage", () => {
    const onAcknowledgedChange = vi.fn();

    render(
      <SlippageRiskNotice
        slippage={5}
        acknowledged={false}
        onAcknowledgedChange={onAcknowledgedChange}
      />,
    );

    const checkbox = screen.getByLabelText(/Acknowledge high slippage risk/i);
    fireEvent.click(checkbox);

    expect(screen.getByText(/High slippage tolerance/i)).toBeInTheDocument();
    expect(onAcknowledgedChange).toHaveBeenCalledWith(true);
  });
});

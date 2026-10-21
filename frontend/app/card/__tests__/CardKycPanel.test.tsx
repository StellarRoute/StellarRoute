import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  CARD_COMPLIANCE_DOC_PATH,
  CARD_KYC_ACK_KEY,
  CardKycPanel,
} from "../CardKycPanel";

describe("CardKycPanel", () => {
  it("renders the gate while the status is kyc_required", () => {
    render(<CardKycPanel status="kyc_required" />);
    expect(screen.getByTestId("card-kyc-panel")).toBeInTheDocument();
    expect(
      screen.getByText(/identity verification is required/i),
    ).toBeInTheDocument();
  });

  it("renders nothing for any other status", () => {
    for (const status of ["kyc_pending", "active", "", null, undefined]) {
      const { unmount } = render(<CardKycPanel status={status} />);
      expect(screen.queryByTestId("card-kyc-panel")).not.toBeInTheDocument();
      unmount();
    }
  });

  it("keeps the continue control disabled without the acknowledgement", () => {
    window.localStorage.clear();
    render(<CardKycPanel status="kyc_required" />);
    expect(screen.getByTestId("card-kyc-continue")).toBeDisabled();
  });

  it("enables continue once the acknowledgement is ticked", () => {
    window.localStorage.clear();
    render(<CardKycPanel status="kyc_required" />);
    fireEvent.click(screen.getByTestId("card-kyc-ack"));
    expect(screen.getByTestId("card-kyc-continue")).toBeEnabled();
  });

  it("stores the acknowledgement locally and restores it on mount", () => {
    window.localStorage.clear();
    const { unmount } = render(<CardKycPanel status="kyc_required" />);
    fireEvent.click(screen.getByTestId("card-kyc-ack"));
    expect(window.localStorage.getItem(CARD_KYC_ACK_KEY)).toBe("1");
    unmount();

    render(<CardKycPanel status="kyc_required" />);
    expect(screen.getByTestId("card-kyc-ack")).toBeChecked();
    expect(screen.getByTestId("card-kyc-continue")).toBeEnabled();
  });

  it("unticking clears the stored acknowledgement", () => {
    window.localStorage.clear();
    render(<CardKycPanel status="kyc_required" />);
    const box = screen.getByTestId("card-kyc-ack");
    fireEvent.click(box);
    fireEvent.click(box);
    expect(window.localStorage.getItem(CARD_KYC_ACK_KEY)).toBeNull();
    expect(screen.getByTestId("card-kyc-continue")).toBeDisabled();
  });

  it("renders no file input of any kind", () => {
    const { container } = render(<CardKycPanel status="kyc_required" />);
    expect(container.querySelector('input[type="file"]')).toBeNull();
    expect(
      screen.queryByLabelText(/upload|attach|document|proof|id file/i),
    ).not.toBeInTheDocument();
  });

  it("never asks for a card number or other card credentials", () => {
    const { container } = render(<CardKycPanel status="kyc_required" />);
    const text = (container.textContent ?? "").toLowerCase();
    for (const forbidden of [
      "card number",
      "card number)",
      "cvv",
      "cvc",
      "expiry",
      "expiration date",
      "pan",
    ]) {
      expect(text).not.toContain(forbidden);
    }
    // The only input on the panel is the acknowledgement checkbox.
    const inputs = container.querySelectorAll("input");
    expect(inputs).toHaveLength(1);
    expect(inputs[0].getAttribute("type")).toBe("checkbox");
  });

  it("links the compliance doc", () => {
    render(<CardKycPanel status="kyc_required" />);
    const link = screen.getByTestId("card-kyc-doc-link");
    expect(link).toHaveAttribute("href", CARD_COMPLIANCE_DOC_PATH);
    expect(link).toHaveAttribute("target", "_blank");
    expect(link.getAttribute("rel")).toContain("noopener");
  });

  it("accepts a compliance doc override", () => {
    render(
      <CardKycPanel status="kyc_required" complianceDocHref="/docs/card" />,
    );
    expect(screen.getByTestId("card-kyc-doc-link")).toHaveAttribute(
      "href",
      "/docs/card",
    );
  });
});

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { OnboardingChecklist } from "./OnboardingChecklist";

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => cleanup());

describe("OnboardingChecklist", () => {
  it("renders all 4 steps", () => {
    render(<OnboardingChecklist />);
    expect(screen.getByText("Connect your wallet")).toBeInTheDocument();
    expect(screen.getByText("Pick a trading pair")).toBeInTheDocument();
    expect(screen.getByText("Review price impact")).toBeInTheDocument();
    expect(screen.getByText("Confirm your swap")).toBeInTheDocument();
  });

  it("dismisses when X button is clicked and persists to localStorage", () => {
    render(<OnboardingChecklist />);
    fireEvent.click(screen.getByRole("button", { name: /dismiss onboarding checklist/i }));
    expect(screen.queryByText("Connect your wallet")).not.toBeInTheDocument();
    expect(localStorage.getItem("stellarroute:onboarding:dismissed")).toBe("true");
  });

  it("dismisses when skip button is clicked", () => {
    render(<OnboardingChecklist />);
    fireEvent.click(screen.getByRole("button", { name: /skip/i }));
    expect(screen.queryByText("Connect your wallet")).not.toBeInTheDocument();
  });

  it("does not render when already dismissed in localStorage", () => {
    localStorage.setItem("stellarroute:onboarding:dismissed", "true");
    render(<OnboardingChecklist />);
    expect(screen.queryByText("Connect your wallet")).not.toBeInTheDocument();
  });

  it("marks completed steps visually", () => {
    render(<OnboardingChecklist completedSteps={["connect_wallet"]} />);
    expect(screen.getByLabelText(/connect your wallet \(completed\)/i)).toBeInTheDocument();
  });

  it("has accessible section label", () => {
    render(<OnboardingChecklist />);
    expect(screen.getByRole("region", { name: /getting started checklist/i })).toBeInTheDocument();
  });
});

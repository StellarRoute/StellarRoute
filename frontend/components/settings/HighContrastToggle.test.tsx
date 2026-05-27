import { describe, expect, it, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { SettingsProvider } from "@/components/providers/settings-provider";
import { HighContrastToggle } from "@/components/settings/HighContrastToggle";

function Wrapper({ children }: { children: React.ReactNode }) {
  return <SettingsProvider>{children}</SettingsProvider>;
}

describe("HighContrastToggle", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.classList.remove("high-contrast");
  });

  it("renders with label and description", () => {
    render(
      <Wrapper>
        <HighContrastToggle />
      </Wrapper>,
    );

    expect(screen.getByLabelText("Toggle high contrast mode")).toBeInTheDocument();
    expect(screen.getByText("High Contrast Mode")).toBeInTheDocument();
    expect(screen.getByText(/increases color contrast/i)).toBeInTheDocument();
  });

  it("is unchecked by default", () => {
    render(
      <Wrapper>
        <HighContrastToggle />
      </Wrapper>,
    );

    const toggle = screen.getByRole("switch", { name: "Toggle high contrast mode" });
    expect(toggle).toHaveAttribute("data-state", "unchecked");
  });

  it("toggles on when clicked and applies class to <html>", async () => {
    render(
      <Wrapper>
        <HighContrastToggle />
      </Wrapper>,
    );

    const toggle = screen.getByRole("switch", { name: "Toggle high contrast mode" });
    await userEvent.click(toggle);

    expect(toggle).toHaveAttribute("data-state", "checked");
    expect(document.documentElement.classList.contains("high-contrast")).toBe(true);
  });

  it("toggles off when clicked again and removes class from <html>", async () => {
    render(
      <Wrapper>
        <HighContrastToggle />
      </Wrapper>,
    );

    const toggle = screen.getByRole("switch", { name: "Toggle high contrast mode" });
    await userEvent.click(toggle);
    await userEvent.click(toggle);

    expect(toggle).toHaveAttribute("data-state", "unchecked");
    expect(document.documentElement.classList.contains("high-contrast")).toBe(false);
  });

  it("persists preference to localStorage", async () => {
    render(
      <Wrapper>
        <HighContrastToggle />
      </Wrapper>,
    );

    const toggle = screen.getByRole("switch", { name: "Toggle high contrast mode" });
    await userEvent.click(toggle);

    const stored = JSON.parse(window.localStorage.getItem("stellar_route_settings") ?? "{}");
    expect(stored.highContrast).toBe(true);
  });
});

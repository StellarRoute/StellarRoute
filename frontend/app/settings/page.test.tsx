import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SettingsProvider } from "@/components/providers/settings-provider";
import SettingsPage from "./page";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock("@/hooks/useBrowserNotifications", () => ({
  useBrowserNotifications: () => ({
    browserNotifications: false,
    permissionState: "default",
    isDisabled: false,
    enableNotifications: vi.fn(),
    disableNotifications: vi.fn(),
  }),
}));

describe("Settings page", () => {
  it("renders the existing settings heading without extra header chrome", () => {
    render(
      <SettingsProvider>
        <SettingsPage />
      </SettingsProvider>,
    );

    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });
});

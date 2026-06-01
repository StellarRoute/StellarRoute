import { vi } from "vitest";

// Mock ExpertSettings completely to isolate the undefined component error
vi.mock("./ExpertSettings", () => ({
  ExpertSettings: () => <div data-testid="mock-expert-settings">Mock Expert Settings</div>
}));

import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, afterEach } from "vitest";
import { SettingsPanel } from "./SettingsPanel";

describe("SettingsPanel Drawer", () => {
  afterEach(() => {
    cleanup();
    localStorage.clear();
  });

  const defaultProps = {
    slippage: 0.5,
    onSlippageChange: vi.fn(),
    deadline: 30,
    onDeadlineChange: vi.fn(),
    expertMode: false,
    bypassConfirmation: false,
    extendedRouteDetails: false,
    onExpertModeChange: vi.fn(),
    onBypassConfirmationChange: vi.fn(),
    onExtendedRouteDetailsChange: vi.fn(),
    onReset: vi.fn(),
    browserNotifications: false,
    notificationPermissionState: 'default' as NotificationPermission,
    notificationsDisabled: false,
    onEnableNotifications: vi.fn().mockResolvedValue(undefined),
    onDisableNotifications: vi.fn(),
  };

  it("renders trigger button successfully", () => {
    render(<SettingsPanel {...defaultProps} />);
    const trigger = screen.getByRole("button", { name: /settings/i });
    expect(trigger).toBeInTheDocument();
  });

  it("opens drawer when settings trigger is clicked", async () => {
    const user = userEvent.setup();
    render(<SettingsPanel {...defaultProps} />);
    
    const trigger = screen.getByRole("button", { name: /settings/i });
    await user.click(trigger);

    // Verify title and content rendered
    expect(screen.getByText("Advanced Settings")).toBeInTheDocument();
    expect(screen.getByText("Slippage Tolerance")).toBeInTheDocument();
    expect(screen.getByText("Transaction Deadline")).toBeInTheDocument();
  });

  it("calls onReset when reset button is clicked inside drawer", async () => {
    const onReset = vi.fn();
    const user = userEvent.setup();
    render(<SettingsPanel {...defaultProps} onReset={onReset} />);
    
    const trigger = screen.getByRole("button", { name: /settings/i });
    await user.click(trigger);

    const resetBtn = screen.getByRole("button", { name: /reset/i });
    await user.click(resetBtn);
    expect(onReset).toHaveBeenCalled();
  });
});

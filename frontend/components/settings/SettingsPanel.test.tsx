import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SettingsProvider } from "@/components/providers/settings-provider";
import { SettingsPanel } from "./SettingsPanel";

function renderPanel() {
  return render(
    <SettingsProvider>
      <SettingsPanel
        slippage={0.5}
        deadline={300}
        expertMode={false}
        bypassConfirmation={false}
        extendedRouteDetails={false}
        onSlippageChange={vi.fn()}
        onDeadlineChange={vi.fn()}
        onExpertModeChange={vi.fn()}
        onBypassConfirmationChange={vi.fn()}
        onExtendedRouteDetailsChange={vi.fn()}
        onReset={vi.fn()}
      />
    </SettingsProvider>,
  );
}

describe("SettingsPanel", () => {
  it("opens the existing settings popover from the toolbar button", async () => {
    const user = userEvent.setup();
    renderPanel();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByTestId("settings-panel")).toBeInTheDocument();
    expect(screen.getByText("Advanced Settings")).toBeInTheDocument();
  });
});

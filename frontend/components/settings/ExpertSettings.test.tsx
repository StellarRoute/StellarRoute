import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SettingsProvider } from "@/components/providers/settings-provider";
import { ExpertSettings } from "./ExpertSettings";

describe("ExpertSettings", () => {
  it("renders under the settings provider", () => {
    render(
      <SettingsProvider>
        <ExpertSettings
          expertMode={false}
          bypassConfirmation={false}
          extendedRouteDetails={false}
          onExpertModeChange={vi.fn()}
          onBypassConfirmationChange={vi.fn()}
          onExtendedRouteDetailsChange={vi.fn()}
        />
      </SettingsProvider>,
    );

    expect(screen.getByRole("switch", { name: "Expert Mode" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
  });
});

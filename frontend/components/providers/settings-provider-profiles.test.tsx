import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsProvider, useSettings } from "@/components/providers/settings-provider";

function TestConsumer() {
  const { settings, addProfile, updateProfile, deleteProfile, selectProfile } = useSettings();

  return (
    <>
      <div data-testid="slippage">{settings.slippageTolerance}</div>
      <div data-testid="active-profile">{settings.activeProfileId}</div>
      <div data-testid="profiles-count">{settings.slippageProfiles.length}</div>
      <button onClick={() => addProfile({ name: 'Custom', value: 2 })}>Add Profile</button>
      <button onClick={() => selectProfile('safe')}>Select Safe</button>
      <button onClick={() => {
        const custom = settings.slippageProfiles.find(p => !p.isPreset);
        if (custom) deleteProfile(custom.id);
      }}>Delete Custom</button>
    </>
  );
}

describe("SettingsProvider Profiles", () => {
  it("manages slippage profiles correctly", async () => {
    window.localStorage.clear();

    render(
      <SettingsProvider>
        <TestConsumer />
      </SettingsProvider>,
    );

    // Initial state
    expect(screen.getByTestId("profiles-count").textContent).toBe("3");
    expect(screen.getByTestId("active-profile").textContent).toBe("balanced");
    expect(screen.getByTestId("slippage").textContent).toBe("0.5");

    // Add profile
    await userEvent.click(screen.getByText("Add Profile"));
    expect(screen.getByTestId("profiles-count").textContent).toBe("4");

    // Select profile
    await userEvent.click(screen.getByText("Select Safe"));
    expect(screen.getByTestId("active-profile").textContent).toBe("safe");
    expect(screen.getByTestId("slippage").textContent).toBe("0.1");

    // Delete custom profile
    await userEvent.click(screen.getByText("Delete Custom"));
    expect(screen.getByTestId("profiles-count").textContent).toBe("3");
  });
});

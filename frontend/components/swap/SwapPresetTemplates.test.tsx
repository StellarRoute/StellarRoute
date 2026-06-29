import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { SwapPresetTemplates } from "./SwapPresetTemplates";
import { DEFAULT_SWAP_PRESETS } from "@/lib/presets";

describe("SwapPresetTemplates", () => {
  beforeEach(() => {
    localStorage.clear();
    cleanup();
  });

  it("renders default presets", () => {
    render(<SwapPresetTemplates onSelect={() => {}} />);
    
    expect(screen.getByText("Quick Pairs")).toBeDefined();
    DEFAULT_SWAP_PRESETS.forEach((preset) => {
      expect(screen.getByText(preset.label)).toBeDefined();
    });
  });

  it("calls onSelect when a preset is clicked", () => {
    const onSelect = vi.fn();
    render(<SwapPresetTemplates onSelect={onSelect} />);
    
    const firstPreset = DEFAULT_SWAP_PRESETS[0];
    const button = screen.getByText(firstPreset.label);
    fireEvent.click(button);
    
    expect(onSelect).toHaveBeenCalledWith(firstPreset.baseAsset, firstPreset.quoteAsset);
  });

  it("highlights the selected preset", () => {
    const selected = DEFAULT_SWAP_PRESETS[0];
    render(
      <SwapPresetTemplates 
        onSelect={() => {}} 
        selectedBase={selected.baseAsset}
        selectedQuote={selected.quoteAsset}
      />
    );
    
    const button = screen.getByText(selected.label);
    expect(button.className).toContain("bg-primary/15");
  });

  it("adds preset to recent tracking when clicked", () => {
    render(<SwapPresetTemplates onSelect={() => {}} />);
    
    const firstPreset = DEFAULT_SWAP_PRESETS[0];
    fireEvent.click(screen.getByText(firstPreset.label));
    
    // Check localStorage
    const stored = JSON.parse(localStorage.getItem("stellar-route-recent-templates") || "[]");
    expect(stored[0].id).toBe(firstPreset.id);
  });
});

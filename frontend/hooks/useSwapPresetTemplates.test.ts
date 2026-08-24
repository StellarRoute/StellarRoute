import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useSwapPresetTemplates } from "./useSwapPresetTemplates";
import { DEFAULT_SWAP_PRESETS } from "@/lib/presets";

describe("useSwapPresetTemplates", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("should initialize with default templates", () => {
    const { result } = renderHook(() => useSwapPresetTemplates());
    
    // Initial templates should include defaults
    expect(result.current.templates.length).toBeGreaterThanOrEqual(DEFAULT_SWAP_PRESETS.length);
    expect(result.current.recentTemplates).toEqual([]);
  });

  it("should add a template to recent ones", () => {
    const { result } = renderHook(() => useSwapPresetTemplates());
    const testPreset = DEFAULT_SWAP_PRESETS[0];

    act(() => {
      result.current.addRecentTemplate(testPreset);
    });

    expect(result.current.recentTemplates).toHaveLength(1);
    expect(result.current.recentTemplates[0].id).toBe(testPreset.id);
    
    // LocalStorage should be updated
    const stored = JSON.parse(localStorage.getItem("stellar-route-recent-templates") || "[]");
    expect(stored).toHaveLength(1);
    expect(stored[0].id).toBe(testPreset.id);
  });

  it("should move existing template to the front when reused", () => {
    const { result } = renderHook(() => useSwapPresetTemplates());
    const preset1 = DEFAULT_SWAP_PRESETS[0];
    const preset2 = DEFAULT_SWAP_PRESETS[1];

    act(() => {
      result.current.addRecentTemplate(preset1);
      result.current.addRecentTemplate(preset2);
    });

    expect(result.current.recentTemplates[0].id).toBe(preset2.id);

    act(() => {
      result.current.addRecentTemplate(preset1);
    });

    expect(result.current.recentTemplates[0].id).toBe(preset1.id);
    expect(result.current.recentTemplates).toHaveLength(2);
  });

  it("should limit the number of recent templates", () => {
    const { result } = renderHook(() => useSwapPresetTemplates());
    
    // Mock 5 templates
    const mockTemplates = Array.from({ length: 6 }, (_, i) => ({
      id: `t${i}`,
      label: `T${i}`,
      baseAsset: "native",
      quoteAsset: `q${i}`,
    }));

    act(() => {
      mockTemplates.forEach(t => result.current.addRecentTemplate(t));
    });

    expect(result.current.recentTemplates).toHaveLength(4); // MAX_RECENT_TEMPLATES = 4
    expect(result.current.recentTemplates[0].id).toBe("t5");
  });
});

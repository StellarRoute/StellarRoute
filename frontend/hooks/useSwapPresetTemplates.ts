"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import { SwapPreset, DEFAULT_SWAP_PRESETS } from "@/lib/presets";

const RECENT_TEMPLATES_KEY = "stellar-route-recent-templates";
const MAX_RECENT_TEMPLATES = 4;

export function useSwapPresetTemplates() {
  const [recentTemplates, setRecentTemplates] = useState<SwapPreset[]>([]);
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    const stored = localStorage.getItem(RECENT_TEMPLATES_KEY);
    if (stored) {
      try {
        setRecentTemplates(JSON.parse(stored));
      } catch (e) {
        console.error("Failed to parse recent templates", e);
      }
    }
    setIsLoaded(true);
  }, []);

  const addRecentTemplate = useCallback((template: SwapPreset) => {
    setRecentTemplates((prev) => {
      // Remove if already exists to move to front
      const filtered = prev.filter((t) => t.id !== template.id);
      const next = [template, ...filtered].slice(0, MAX_RECENT_TEMPLATES);
      localStorage.setItem(RECENT_TEMPLATES_KEY, JSON.stringify(next));
      return next;
    });
  }, []);

  // Combined list for the UI
  const allTemplates = useMemo(() => {
    // Start with recent ones, then add defaults that aren't in recent
    const recentIds = new Set(recentTemplates.map((t) => t.id));
    const uniqueDefaults = DEFAULT_SWAP_PRESETS.filter(
      (d) => !recentIds.has(d.id)
    );
    
    return [...recentTemplates, ...uniqueDefaults];
  }, [recentTemplates]);

  return { 
    templates: allTemplates, 
    recentTemplates,
    addRecentTemplate, 
    isLoaded 
  };
}

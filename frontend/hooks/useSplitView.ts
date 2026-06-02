"use client";

import { useState, useCallback } from "react";

const SPLIT_VIEW_KEY = "stellar-route-split-view";

function loadSplitView(): boolean {
  if (typeof window === "undefined") return false;
  return localStorage.getItem(SPLIT_VIEW_KEY) === "true";
}

export function useSplitView() {
  const [isSplit, setIsSplit] = useState(loadSplitView);

  const toggleSplit = useCallback(() => {
    setIsSplit((prev) => {
      const next = !prev;
      localStorage.setItem(SPLIT_VIEW_KEY, String(next));
      return next;
    });
  }, []);

  return { isSplit, toggleSplit };
}

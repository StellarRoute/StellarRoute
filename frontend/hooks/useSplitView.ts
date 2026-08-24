"use client";

import { useCallback, useState, useSyncExternalStore } from "react";

const SPLIT_VIEW_KEY = "stellar-route-split-view";

function subscribeToSplitView(onStoreChange: () => void) {
  window.addEventListener("storage", onStoreChange);
  return () => window.removeEventListener("storage", onStoreChange);
}

function getSplitViewSnapshot(): boolean {
  return localStorage.getItem(SPLIT_VIEW_KEY) === "true";
}

function getSplitViewServerSnapshot(): boolean {
  return false;
}

export function useSplitView() {
  const storedSplit = useSyncExternalStore(
    subscribeToSplitView,
    getSplitViewSnapshot,
    getSplitViewServerSnapshot,
  );
  const [localSplit, setLocalSplit] = useState<boolean | null>(null);
  const isSplit = localSplit ?? storedSplit;

  const toggleSplit = useCallback(() => {
    const next = !isSplit;
    localStorage.setItem(SPLIT_VIEW_KEY, String(next));
    setLocalSplit(next);
  }, [isSplit]);

  return { isSplit, toggleSplit };
}

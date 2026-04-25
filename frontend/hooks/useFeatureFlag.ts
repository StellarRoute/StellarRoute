"use client";

import { useSyncExternalStore } from "react";

import {
  getFeatureFlag,
  type FeatureFlagName,
} from "@/lib/feature-flags";

function subscribe(): () => void {
  return () => {};
}

export function useFeatureFlag(name: FeatureFlagName): boolean {
  return useSyncExternalStore(
    subscribe,
    () => getFeatureFlag(name),
    () => getFeatureFlag(name),
  );
}

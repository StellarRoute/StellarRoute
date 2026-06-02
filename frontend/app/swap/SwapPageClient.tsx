"use client";

import { SwapCard } from "@/components/swap/SwapCard";
import { SplitView } from "@/components/swap/SplitView";
import { useSplitView } from "@/hooks/useSplitView";
import dynamic from "next/dynamic";

const RouteDisplay = dynamic(
  () => import("@/components/swap/RouteDisplay").then((m) => m.RouteDisplay),
  { ssr: false }
);

export function SwapPageClient() {
  const { isSplit, toggleSplit } = useSplitView();

  return (
    <SplitView
      isSplit={isSplit}
      onToggle={toggleSplit}
      primary={<SwapCard />}
      secondary={
        <div className="rounded-xl border border-border/50 bg-card p-4">
          <h2 className="text-sm font-semibold mb-3">Route Details</h2>
          <RouteDisplay amountOut="0" />
        </div>
      }
      className="w-full max-w-[960px] mx-auto"
    />
  );
}

"use client";

import { useFeatureFlag } from "@/hooks/useFeatureFlag";

interface RoutesBetaGateProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

export function RoutesBetaGate({ children, fallback = null }: RoutesBetaGateProps) {
  const { enabled } = useFeatureFlag("routes_beta");
  return enabled ? <>{children}</> : <>{fallback}</>;
}


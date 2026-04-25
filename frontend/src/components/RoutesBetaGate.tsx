/**
 * RoutesBetaGate
 *
 * Example of a feature-flagged component.
 * Renders the beta routes UI only when "routes_beta" flag is enabled.
 * Shows nothing (or a fallback) while the flag is loading.
 */

import { useFeatureFlag } from "../hooks/useFeatureFlag";

interface RoutesBetaGateProps {
  /** Rendered when the flag is ON */
  children: React.ReactNode;
  /** Optional fallback rendered when flag is OFF (default: null) */
  fallback?: React.ReactNode;
}

export function RoutesBetaGate({
  children,
  fallback = null,
}: RoutesBetaGateProps) {
  const { enabled, loading } = useFeatureFlag("routes_beta");

  // Don't flash content while resolving
  if (loading) return null;

  return enabled ? <>{children}</> : <>{fallback}</>;
}

// ─── Usage example ────────────────────────────────────────────────────────────
//
// import { RoutesBetaGate } from "@/components/RoutesBetaGate";
//
// export default function SwapPage() {
//   return (
//     <RoutesBetaGate fallback={<LegacyRoutes />}>
//       <RoutesBeta />
//     </RoutesBetaGate>
//   );
// }

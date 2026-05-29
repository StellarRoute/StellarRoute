"use client";

/**
 * Developer-only debug overlay (issue #517).
 *
 * Displays quote IDs, snapshot versions, and performance timings to help
 * developers diagnose issues during development. Wallet addresses are
 * masked so sensitive information is never exposed.
 *
 * ACTIVATION (dev builds only):
 *  - Keyboard shortcut: Ctrl+Shift+D  (Cmd+Shift+D on macOS)
 *  - Query parameter:   ?debug=1
 *
 * The component renders nothing in production builds
 * (process.env.NODE_ENV === 'production').
 */

import { useEffect, useState, useCallback } from "react";
import { useSearchParams } from "next/navigation";

export interface DebugInfo {
  /** Quote ID currently displayed in the panel */
  quoteId?: string;
  /** Snapshot / data-version identifier */
  snapshotVersion?: string;
  /** Arbitrary key→milliseconds timing map */
  timings?: Record<string, number>;
}

interface DebugOverlayProps {
  /** Debug data pushed in from the parent page / context */
  info?: DebugInfo;
}

/** Mask all but the first 4 and last 4 chars of a wallet address. */
function maskAddress(address: string): string {
  if (address.length <= 10) return "****";
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

/**
 * DebugOverlay — dev-only floating panel.
 *
 * Rendered only when `process.env.NODE_ENV !== 'production'`.
 * The panel is hidden by default; toggle with Ctrl/Cmd+Shift+D or ?debug=1.
 */
export function DebugOverlay({ info = {} }: DebugOverlayProps) {
  // Hard gate: never render in production.
  if (process.env.NODE_ENV === "production") return null;

  return <DebugOverlayInner info={info} />;
}

/** Inner component (only mounted in non-production environments). */
function DebugOverlayInner({ info }: { info: DebugInfo }) {
  const searchParams = useSearchParams();
  const queryEnabled =
    searchParams?.get("debug") === "1" ||
    searchParams?.get("debug") === "true";

  const [visible, setVisible] = useState(queryEnabled);
  const [perfNow] = useState(() => performance.now());

  // Toggle via Ctrl+Shift+D (Windows/Linux) or Cmd+Shift+D (macOS)
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "D" && e.shiftKey && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        setVisible((v) => !v);
      }
    },
    []
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  // Sync with query param changes (e.g. manual URL edits)
  useEffect(() => {
    if (queryEnabled) setVisible(true);
  }, [queryEnabled]);

  if (!visible) return null;

  const { quoteId, snapshotVersion, timings } = info;

  return (
    <div
      role="complementary"
      aria-label="Developer debug overlay"
      data-testid="debug-overlay"
      style={{
        position: "fixed",
        bottom: "1rem",
        right: "1rem",
        zIndex: 9999,
        maxWidth: "320px",
        width: "100%",
        fontFamily: "monospace",
        fontSize: "11px",
        lineHeight: "1.5",
        backgroundColor: "rgba(0,0,0,0.85)",
        color: "#86efac",   // green-300 — readable on dark bg, never confused with UI
        border: "1px solid rgba(134,239,172,0.3)",
        borderRadius: "6px",
        padding: "10px 12px",
        backdropFilter: "blur(4px)",
        pointerEvents: "auto",
        userSelect: "text",
      }}
    >
      {/* Header row */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "6px",
          borderBottom: "1px solid rgba(134,239,172,0.2)",
          paddingBottom: "4px",
        }}
      >
        <span style={{ fontWeight: "bold", color: "#a3e635" }}>
          🛠 DEV OVERLAY
        </span>
        <button
          onClick={() => setVisible(false)}
          aria-label="Close debug overlay"
          style={{
            background: "transparent",
            border: "none",
            color: "#86efac",
            cursor: "pointer",
            fontSize: "13px",
            lineHeight: 1,
            padding: "0 2px",
          }}
        >
          ✕
        </button>
      </div>

      {/* Quote ID */}
      <Row label="Quote ID" value={quoteId ?? "—"} />

      {/* Snapshot version */}
      <Row label="Snapshot" value={snapshotVersion ?? "—"} />

      {/* Performance timings */}
      {timings && Object.keys(timings).length > 0 ? (
        <>
          <div style={{ marginTop: "6px", color: "#a3e635", fontWeight: "bold" }}>
            Timings
          </div>
          {Object.entries(timings).map(([key, ms]) => (
            <Row key={key} label={key} value={`${ms.toFixed(1)} ms`} />
          ))}
        </>
      ) : (
        <Row label="Timings" value="none recorded" />
      )}

      {/* Overlay mount age — useful for detecting re-mounts */}
      <Row
        label="Overlay age"
        value={`${(performance.now() - perfNow).toFixed(0)} ms`}
      />

      {/* Tip */}
      <div
        style={{
          marginTop: "6px",
          color: "#6b7280",
          fontSize: "10px",
          borderTop: "1px solid rgba(134,239,172,0.1)",
          paddingTop: "4px",
        }}
      >
        Toggle: Ctrl/Cmd+Shift+D  ·  ?debug=1
        <br />
        Wallet addresses are masked for security.
        <br />
        Hidden in production builds.
      </div>
    </div>
  );
}

/** Compact label–value row. */
function Row({ label, value }: { label: string; value: string }) {
  const safeValue = looksLikeAddress(value) ? maskAddress(value) : value;
  return (
    <div style={{ display: "flex", gap: "4px", justifyContent: "space-between" }}>
      <span style={{ color: "#9ca3af", flexShrink: 0 }}>{label}:</span>
      <span
        style={{
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          maxWidth: "200px",
          textAlign: "right",
        }}
        title={safeValue}
      >
        {safeValue}
      </span>
    </div>
  );
}

/** Heuristic: Stellar public keys are 56-char base32 strings starting with G */
function looksLikeAddress(s: string): boolean {
  return /^G[A-Z2-7]{55}$/.test(s);
}

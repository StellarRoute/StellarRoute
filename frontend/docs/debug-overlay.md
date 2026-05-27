# Developer Debug Overlay

> Issue [#517](https://github.com/StellarRoute/StellarRoute/issues/517)

A floating dev-only panel that displays diagnostic data (quote IDs, snapshot versions, and performance timings) to help developers diagnose routing and data issues without touching prod users.

---

## Activation

| Method | How |
|--------|-----|
| **Keyboard shortcut** | `Ctrl + Shift + D` (Windows / Linux) or `Cmd + Shift + D` (macOS) — toggles the panel on / off |
| **Query parameter** | Append `?debug=1` (or `?debug=true`) to any URL — panel opens automatically on load |

---

## What is displayed

| Field | Description |
|-------|-------------|
| **Quote ID** | Unique identifier of the quote currently shown in the panel |
| **Snapshot** | Version/revision of the price snapshot used for the current quote |
| **Timings** | Arbitrary `key → milliseconds` entries (e.g. `quoteLatency`, `renderTime`) |
| **Overlay age** | How many milliseconds the panel component has been mounted — useful to detect unexpected remounts |

---

## Security

* **Stellar wallet addresses** (56-character base32 strings starting with `G`) are automatically **masked** to `GABC…WXYZ` format — the first 4 and last 4 characters only.
* No other wallet data (private keys, seed phrases, balances) is ever passed to or displayed by this component.
* The component **hard-gates** on `process.env.NODE_ENV === 'production'` and returns `null` before rendering anything, so **it is impossible for the overlay to appear in production builds**.

---

## Usage in a page / component

```tsx
import { DebugOverlay } from '@/components/debug/DebugOverlay';

// Minimal usage — panel is toggled by keyboard / query param
<DebugOverlay />

// With data
<DebugOverlay
  info={{
    quoteId: quote.id,
    snapshotVersion: snapshot.version,
    timings: {
      quoteLatency: performance.now() - fetchStart,
      renderTime: performance.now() - renderStart,
    },
  }}
/>
```

The overlay is already mounted inside `AppShell` so it is available on every page. You only need to provide the `info` prop if you want to surface page-specific data.

---

## Disabling the overlay

The overlay is development-only. No action is needed for production — the build-time `NODE_ENV` guard ensures zero overhead.

To prevent it loading in a specific test or Storybook story, either:

1. Mock `process.env.NODE_ENV` to `'production'`.
2. Wrap the component with a feature flag check before rendering.

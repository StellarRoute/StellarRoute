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

### Automatic wiring (swap flows)

When using the swap feature, quote metadata (ID, snapshot version, timings) is **automatically** wired from the `useQuote` hook through `SwapCard` into the debug overlay via a React context (`DebugOverlayContext`). No explicit prop passing is required.

The `useQuote` hook extracts:
- **`quoteId`**: Unique identifier for each quote (generated from timestamp + random suffix)
- **`snapshotVersion`**: Data source version (extracted from the quote's `source_timestamp` or `timestamp`)
- **`timings`**: Performance metrics (currently empty, but ready for future instrumentation)

```tsx
// In SwapCard — automatically populated
const { setDebugInfo } = useDebugOverlay();
useEffect(() => {
  setDebugInfo({
    quoteId: quote.quoteId,
    snapshotVersion: quote.snapshotVersion,
    timings: quote.timings,
  });
}, [quote.quoteId, quote.snapshotVersion, quote.timings, setDebugInfo]);
```

### Manual usage (other pages/components)

For non-swap pages, you can manually provide data to the debug overlay:

```tsx
import { DebugOverlay } from '@/components/debug/DebugOverlay';
import { useDebugOverlay } from '@/contexts/DebugOverlayContext';

export function MyComponent() {
  const { setDebugInfo } = useDebugOverlay();

  useEffect(() => {
    setDebugInfo({
      quoteId: myQuoteId,
      snapshotVersion: mySnapshotVersion,
      timings: {
        quoteLatency: performance.now() - fetchStart,
        renderTime: performance.now() - renderStart,
      },
    });
  }, []);

  return <div>My component</div>;
}
```

The overlay is already mounted inside `AppShell` and wrapped with `DebugOverlayProvider` so it is available on every page.

---

## Disabling the overlay

The overlay is development-only. No action is needed for production — the build-time `NODE_ENV` guard ensures zero overhead.

To prevent it loading in a specific test or Storybook story, either:

1. Mock `process.env.NODE_ENV` to `'production'`.
2. Wrap the component with a feature flag check before rendering.

# Quote Refresh Screen Reader Testing

Manual verification notes for issue #513 — live region announcements during quote refresh.

## Scope

- `SwapCard` (`/swap`) — primary swap UI
- `DemoSwap` (demo page using `useQuoteRefresh`) — debounced amount entry

Announcements are rendered by `QuoteRefreshLiveRegion` and driven by `useQuoteRefreshAnnouncements`.

## Prerequisites

1. Start the frontend: `cd frontend && npm run dev`
2. Use a screen reader:
   - **Windows**: NVDA (free) or JAWS
   - **macOS**: VoiceOver (`Cmd + F5`)
3. Ensure the quote API is reachable (or use mocked/demo data as documented for local dev)

## Expected behavior

| Scenario | Region | Priority | Expected announcement |
| --- | --- | --- | --- |
| Quote fetch succeeds (manual refresh, auto-refresh, or debounced amount) | `role="status"` | polite | “Quote updated.” or “Quote updated. {rate}” |
| Hard failure (4xx, offline, retries exhausted) | `role="alert"` | assertive | “Quote refresh failed. {message}” |
| Transient failure while auto-retry is pending | — | — | **No** announcement until retries finish |
| Rapid amount edits during debounce | — | — | **One** success announcement per completed quote, not per keystroke |

Focus must remain on the control the user is editing (amount field, refresh button, etc.).

## Test steps

### 1. Polite success on manual refresh

1. Open `/swap` and connect wallet (or use demo with valid pair/amount).
2. Enter a valid pay amount and wait for the initial quote.
3. Tab to **Refresh quote** and activate it.
4. **Pass**: Screen reader announces a polite “Quote updated…” message; focus stays on the refresh control.

### 2. Polite success after debounced typing

1. Focus the pay amount field.
2. Type `1`, then quickly append `0` and `0` (e.g. `100`) without pausing long between keys.
3. **Pass**: After debounce settles and the request completes, exactly **one** polite quote-updated announcement is heard (not three).

### 3. Assertive hard failure

1. Stop the API or force an invalid amount that returns a non-retryable 400.
2. Trigger a quote refresh.
3. **Pass**: Assertive “Quote refresh failed…” is announced; no duplicate failure spam on subsequent renders.

### 4. No announcement during transient retry

1. Simulate a transient 503/429 that auto-retries (or use integration test mocks).
2. **Pass**: No assertive failure during the retry window; polite success after recovery if the retry succeeds.

### 5. Offline

1. Disable network in devtools.
2. Attempt to refresh the quote.
3. **Pass**: Assertive failure mentioning offline/reconnect guidance.

## Automated coverage

```bash
cd frontend
npm test -- lib/quote-refresh-announcements.test.ts hooks/useQuoteRefreshAnnouncements.test.ts components/swap/QuoteRefreshLiveRegion.test.tsx
```

## Sign-off checklist

- [ ] Polite announcement on successful refresh
- [ ] Assertive announcement on hard failure
- [ ] No duplicate announcements during debounce
- [ ] Focus never moves to the live region

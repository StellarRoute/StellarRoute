# Storybook / Ladle for StellarRoute frontend

This repository uses Ladle to render component stories for core swap primitives.

## Available stories
- `SwapCard` state matrix (`components/swap/SwapCard.stories.tsx`) — idle, quoting, stale, confirming, and error fixtures
- `Swap primitives` (Core components in `components/swap/`)
- `TokenSelector` (Shared component)
- `QuoteCard` (Shared component)
- `RouteRow` (Shared component)
- `SlippageControl` (Shared component)

## Run locally
1. `cd frontend`
2. `npm install`
3. `npm run storybook`

## CI command
- `npm run storybook:ci`

The CI workflow runs this in the `Frontend Ladle Storybook` job (`.github/workflows/ci.yml`).
On success, the static `frontend/build/` output is uploaded as the `ladle-storybook` artifact
and verified by `scripts/check-storybook-build.mjs` (index.html present, size under 2 MiB).
Broken stories fail the Ladle build and block CI.

## Swap UI component review checklist

Use this checklist before merging a new or significantly changed swap UI
component. Copy it into the PR description or attach it as a review comment when
the component affects the swap form, route display, quote summary, token pair
selection, settings, or transaction confirmation flow.

### Component metadata

| Field | Value |
| --- | --- |
| Component |  |
| Story or demo route |  |
| Primary user action |  |
| Related issue or PR |  |
| Reviewer |  |

### Accessibility

- [ ] Keyboard navigation reaches every interactive control in a predictable order.
- [ ] Focus states are visible in default, hover, active, and disabled states.
- [ ] Controls have accessible names that describe the action or selected value.
- [ ] Status updates use `role="status"`, `aria-live`, or an equivalent non-disruptive pattern.
- [ ] Error messages are associated with the control or region they describe.
- [ ] Color is not the only way to identify risk, selection, or validation state.

### Loading and error states

- [ ] Loading state preserves layout dimensions and avoids content shift.
- [ ] Empty state explains what data is missing without blocking manual recovery.
- [ ] Error state gives a next action, retry path, or safe fallback.
- [ ] Disabled state explains why the action is unavailable when context is not obvious.
- [ ] Long-running network states do not leave the user with duplicate submit actions.

### Mobile and responsive behavior

- [ ] Component fits at 320 px width without horizontal scrolling.
- [ ] Touch targets are at least 44 x 44 px for primary controls.
- [ ] Dense route, token, or amount text wraps or truncates intentionally.
- [ ] Sticky, modal, drawer, and popover states remain reachable on mobile viewports.
- [ ] Reduced-motion users are not required to rely on animation to understand state.

### Internationalization readiness

- [ ] User-facing strings are centralized or easy to extract.
- [ ] Labels tolerate longer translated text without overlapping adjacent controls.
- [ ] Numbers, token amounts, percentages, and dates use existing formatting helpers.
- [ ] Sentence fragments are not split across multiple hard-coded strings.
- [ ] Direction-sensitive icons or layouts are checked for RTL compatibility when applicable.

### Manual pair selection non-regression

- [ ] Manual base and quote selection still works without using the new component.
- [ ] Existing form values are not overwritten unless the user explicitly selects a new option.
- [ ] Validation still runs for unsupported pairs, stale quotes, and invalid amounts.
- [ ] Recent-token, favorite-token, or preset behavior remains local to the browser.

## Example completed checklist: `QuoteCard`

| Field | Value |
| --- | --- |
| Component | `QuoteCard` |
| Story or demo route | `frontend/components/shared/QuoteCard.tsx` story/demo |
| Primary user action | Review selected quote details before continuing a swap |
| Related issue or PR | Example review entry |
| Reviewer | Frontend reviewer |

### Accessibility

- [x] Quote values are readable as text and do not rely only on color.
- [x] Interactive actions have accessible button text or labels.
- [x] Risk and status copy can be announced by surrounding status regions.

### Loading and error states

- [x] Missing quote data can fall back to the existing empty or loading view.
- [x] Error copy can be rendered outside the card without changing manual selection.

### Mobile and responsive behavior

- [x] Token symbols, amounts, and route metadata wrap within the card width.
- [x] Primary actions remain reachable at narrow mobile widths.

### Internationalization readiness

- [x] Numeric quote fields use formatting helpers instead of raw string concatenation.
- [x] Labels can be moved to translation keys without changing component structure.

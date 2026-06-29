# Responsive Layout Specification

## Breakpoint Reference

All breakpoints match Tailwind v4 defaults (no custom overrides in `frontend/app/globals.css`).

| Prefix | Min Width  | Target Device        |
|--------|------------|----------------------|
| `sm`   | 640px      | Large phones / small tablets |
| `md`   | 768px      | Tablets portrait     |
| `lg`   | 1024px     | Tablets landscape / small desktops |
| `xl`   | 1280px     | Desktops             |
| `2xl`  | 1536px     | Large desktops       |

Layout behavior is specified at `sm`, `md`, `lg`, and `xl` as a progressive enhancement stack — each larger breakpoint inherits from the previous one and overrides specific column/panel rules.

---

## Breakpoint Table: Column Layout & Panel Stacking

### Swap Page (`/swap`) — SplitView

| Breakpoint | Columns | Swap Card | Route Panel | Split View Toggle |
|------------|---------|-----------|-------------|-------------------|
| **< sm** (default) | 1 | Full width (centered via mx-auto), max-w-[480px] | **Hidden**. Accessed via a bottom-sheet trigger button below the swap card | Hidden (not rendered) |
| **sm+** | 1 | Same as default | Same as default | Visible |
| **md+** | 1 (standard) or 2 (split) | max-w-[480px] centered when standard; `flex-1 min-w-0` in split mode | `hidden md:hidden` when standard; `flex-1 min-w-0` rendered inline beside swap card in split mode | Visible, functional |
| **lg+** | 1 or 2 | Same as md+ | Same as md+ | Visible |
| **xl+** | 1 or 2 | Same as md+ | Same as md+ | Visible |

Notes:
- The `useMediaQuery('(min-width: 768px)')` hook in `MobileRouteBottomSheet` controls whether route details render inline (desktop, md+) or as a bottom sheet (mobile, < md).
- The `SplitView` component wraps both panels in a `max-w-[960px]` container.
- For tablet split-view behavior see [Tablet Split-View](#tablet-split-view-behavior) below.

### Orderbook Page (`/orderbook`) — Full-page

| Breakpoint | Columns | Market Depth Chart | Bid Table | Ask Table | Pair Selector |
|------------|---------|-------------------|-----------|-----------|---------------|
| **< sm** | 1 | Full width | Full width, stacked below chart | Full width, stacked below bids | Horizontal scrollable row |
| **sm+** | 1 | Full width | Full width | Full width | Wrap-enabled flex row |
| **md+** | 2 (bids/asks side by side) | Full width | Left column | Right column | Same |
| **lg+** | 2 | Same as md+ | Same as md+ | Same as md+ | Same |
| **xl+** | 2 | Same as md+ | Same as md+ | Same as md+ | Same |

Implementation: `grid gap-4 md:grid-cols-2` on the bid/ask container.

### Orderbook Depth Panel (in-swap context)

| Breakpoint | Layout | Bids | Asks | Divider |
|------------|--------|------|------|---------|
| **< sm** | Stacked + horizontally scrollable | Top | Bottom | Hidden |
| **sm+** | Side-by-side | Left | Right | Visible (`sm:block w-px bg-border`) |
| **md+** | Side-by-side | Left | Right | Visible |
| **lg+** | Side-by-side | Left | Right | Visible |

Implementation: `flex flex-col gap-4 overflow-x-auto sm:flex-row`.

### App Shell — Content Width

| Breakpoint | Centered Pages (swap, history, settings) | Full-width Pages (orderbook, analytics) |
|------------|------------------------------------------|----------------------------------------|
| **< sm** | `max-w-7xl, px-3` | `w-full` |
| **sm+** | `px-6` | `w-full` |
| **lg+** | `px-8` | `w-full` |

---

## Tablet Split-View Behavior

On tablets (768px–1024px, i.e. `md` breakpoint range), the following rules apply:

1. **Split View toggle is functional.** The button renders in the header area above both panels.
2. **Standard mode (single column):** Same as mobile — swap card at `max-w-[480px]` centered, route panel hidden.
3. **Split mode:** Both panels render side by side with `md:flex-1 md:min-w-0`, sharing horizontal space equally. Each panel compresses to fit.
4. **Route panel:** Because the route panel has no min-width floor in split mode, long route lists may require internal scrolling. `RouteDisplay` uses `useVirtualWindow` to virtualize the route list inside a scroll container.
5. **Navigation:** The hamburger menu is used (`md:hidden` / `hidden md:flex` for desktop nav).

---

## Touch vs Pointer Interaction Differences

### Touch-first behavior (width < 768px)

| Element | Touch Behavior |
|---------|---------------|
| Swap button | Full tap target (min 44px), no hover states on mobile |
| Route panel | Opens as a bottom-drawer `Sheet` with `50vh` default and `94vh` expanded snap point. Drag-to-dismiss supported via Radix Sheet gesture |
| Token selector | Full-screen modal overlay on mobile |
| Pair selector (orderbook) | Horizontal scroll for pairs, larger touch targets on buttons |
| Amount input | Numeric keyboard on focus (`inputMode="decimal"`) |
| Settings panel | Slide-out drawer from right |
| Market depth chart | Pinch-zoom disabled, tap to inspect data point |
| Bid/ask rows | Tap to select price (fills swap input), no hover highlight |

### Pointer-first behavior (width >= 768px)

| Element | Pointer Behavior |
|---------|------------------|
| Swap button | Hover state (`hover:` utility classes). Still satisfies 44px tap target |
| Route panel | Rendered inline as a static column. Row hover states on route options. No bottom-sheet gesture |
| Token selector | Dropdown/popover, dismiss on click-away |
| Bid/ask rows | Hover background highlight (`hover:bg-emerald-500/10` / `hover:bg-red-500/10`), click to select price |
| Orderbook pair selector | Standard button hover states, no scroll wrapper needed |
| Split view toggle | Visible and clickable with hover state |

### Shared rules (all viewports)

- All interactive elements use `cursor-pointer` via `<button>` or explicit class.
- Touch targets must be at least 44x44px as enforced by E2E assertions (see `frontend/e2e/mobile-layout.spec.ts`).
- All `aria-label`, `aria-pressed`, and `role` attributes are applied regardless of input method.

---

## Source Files

| File | Role |
|------|------|
| `frontend/components/swap/SplitView.tsx` | Swap page split/standard layout orchestrator |
| `frontend/components/swap/MobileRouteBottomSheet.tsx` | Mobile bottom-sheet for route details |
| `frontend/components/swap/RouteDisplay.tsx` | Route panel content |
| `frontend/components/swap/OrderbookDepthPanel.tsx` | In-swap compact orderbook |
| `frontend/app/swap/SwapPageClient.tsx` | Swap page entry point with lazy-loaded components |
| `frontend/app/orderbook/OrderbookPageClient.tsx` | Full orderbook page |
| `frontend/components/layout/app-shell.tsx` | Root layout shell |
| `frontend/hooks/useMediaQuery.ts` | Media query hook for JS-driven responsive behavior |
| `frontend/hooks/useSplitView.ts` | Split view toggle state + localStorage persistence |

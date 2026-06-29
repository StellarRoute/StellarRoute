# Empty-State Design System Spec
**Issue:** #461 — Design empty-state patterns for swap, routes, and history surfaces  
**Milestone:** M4 — Web UI  
**Complexity:** Medium  
**Status:** Design Review Ready  

## Overview

This spec defines reusable empty-state patterns across StellarRoute's primary surfaces. All empty states follow the existing `ViewState` component primitive with standardized copy, illustration slots, CTAs, and accessibility guidance.

### Key Principles
- **Consistency:** All surfaces use the same `ViewState` component (`frontend/components/shared/ViewState.tsx`)
- **Progressive Disclosure:** Primary CTA is always visible; secondary help links appear on demand
- **Accessibility First:** Every state includes `role`, `aria-label`, and alt text for icons
- **Instructive:** Copy explains *why* the surface is empty and *what to do next*

---

## ViewState Component (Existing)

### Component API

```tsx
interface ViewStateProps {
  variant: "loading" | "empty" | "error";
  title: string;
  description: string;
  action?: ReactNode;
  className?: string;
}
```

### Current Icon Mapping
- **Loading:** Animated spinner (`Loader2`), `aria-hidden="true"`
- **Empty:** Inbox icon (`Inbox`), `aria-hidden="true"` 
- **Error:** Alert triangle (`AlertTriangle`), `aria-hidden="true"`

### Accessibility Features
- `role="status"` for loading/empty
- `role="alert"` for errors
- Icons marked `aria-hidden="true"` (copy conveys intent)
- Parent role supports ARIA live updates

---

## Surface: Swap Card (Swap Page `/`)

### State 1: Empty Selection (No Pair Selected)

**When:** User first lands on swap page or clears pair selection  
**Copy:**
- **Title:** "Select a trading pair"
- **Description:** "Choose from available base and quote assets to get started."

**CTA:** 
- **Primary:** "Browse pairs" → Opens token selector for base asset
- **Secondary:** Link to docs: "Learn about asset formats"

**Variant:** `empty`

**Implementation:**
```tsx
<ViewState
  variant="empty"
  title="Select a trading pair"
  description="Choose from available base and quote assets to get started."
  action={
    <Button onClick={openPairSelector} variant="default">
      Browse pairs
    </Button>
  }
/>
```

**A11y Requirements:**
- `aria-describedby` references the description text
- Token selector dialog must have `role="dialog"` with `aria-labelledby`
- Alt text if custom illustration: "Illustration of assets for selection"

---

### State 2: Loading Quote

**When:** User enters amount and quote is being fetched  
**Copy:**
- **Title:** "Finding best route"
- **Description:** "Searching for optimal price across Stellar DEX and Soroban pools..."

**CTA:** None (user waits)

**Variant:** `loading`

**Implementation:**
```tsx
<ViewState
  variant="loading"
  title="Finding best route"
  description="Searching for optimal price across Stellar DEX and Soroban pools..."
/>
```

**A11y Requirements:**
- `role="status"` with `aria-live="polite"` (automatically added by component)
- Spinner must have `aria-hidden="true"` (no additional announcement)
- Description conveys loading intent

---

### State 3: No Liquidity for Pair

**When:** Quote fails because no liquidity exists for the selected pair  
**Copy:**
- **Title:** "No liquidity for this pair"
- **Description:** "Try a different trading pair or contact pool operators to add liquidity."

**CTA:**
- **Primary:** "Try different pair" → Reset quote and focus pair selector
- **Secondary:** Link to Stellar docs on pools

**Variant:** `error`

**Implementation:**
```tsx
<ViewState
  variant="error"
  title="No liquidity for this pair"
  description="Try a different trading pair or contact pool operators to add liquidity."
  action={
    <Button onClick={resetPair} variant="outline">
      Try different pair
    </Button>
  }
/>
```

**A11y Requirements:**
- `role="alert"` (automatically added by component)
- Error icon: `aria-hidden="true"`
- Clear explanation in description for screen readers

---

### State 4: Quote Fetch Error (Network)

**When:** API returns error (rate limit, timeout, or server error)  
**Copy:**
- **Title:** "Could not get price quote"
- **Description:** "The pricing service is temporarily unavailable. Please try again."

**CTA:**
- **Primary:** "Retry" → Refetch quote with same inputs
- **Secondary:** "Switch network" if network-specific error

**Variant:** `error`

**Implementation:**
```tsx
<ViewState
  variant="error"
  title="Could not get price quote"
  description="The pricing service is temporarily unavailable. Please try again."
  action={
    <Button onClick={retryQuote} variant="outline">
      Retry
    </Button>
  }
/>
```

**A11y Requirements:**
- Same as "No liquidity" error state
- CTA must be keyboard accessible
- Screen reader announces the error condition

---

## Surface: Route List (Orderbook Page `/orderbook`)

### State 1: No Markets Available

**When:** Indexer has not synced any trading pairs yet  
**Copy:**
- **Title:** "No markets available"
- **Description:** "The indexer is syncing trading pairs. Check back in a few moments."

**CTA:**
- **Primary:** "Refresh" → Reload available pairs
- **Secondary:** Status page link

**Variant:** `empty`

**Implementation:**
```tsx
<ViewState
  variant="empty"
  title="No markets available"
  description="The indexer is syncing trading pairs. Check back in a few moments."
  action={
    <Button onClick={refreshPairs} variant="default">
      Refresh
    </Button>
  }
/>
```

---

### State 2: Loading Markets

**When:** Pairs are being fetched from the indexer  
**Copy:**
- **Title:** "Loading markets"
- **Description:** "Fetching available trading pairs..."

**Variant:** `loading`

**A11y Requirements:**
- `aria-live="polite"` for status updates

---

### State 3: Selected Pair, No Orderbook Depth

**When:** Pair is selected but no bids/asks currently exist  
**Copy:**
- **Title:** "No orderbook entries"
- **Description:** "There are currently no bids or asks for this pair. Check another pair or try later."

**CTA:**
- **Primary:** "Browse other pairs" 
- **Secondary:** "Export as CSV" (if available later)

**Variant:** `empty`

---

### State 4: Orderbook Fetch Error

**When:** API fails to fetch orderbook for selected pair  
**Copy:**
- **Title:** "Could not load orderbook"
- **Description:** "Try refreshing or selecting a different pair."

**CTA:**
- **Primary:** "Retry" → Refetch same pair
- **Secondary:** "Select different pair"

**Variant:** `error`

---

## Surface: Transaction History (History Page `/history`)

### State 1: No Transactions (Fresh Wallet)

**When:** User first connects wallet with no swap history  
**Copy:**
- **Title:** "No transactions yet"
- **Description:** "You haven't made any swaps. Head to the Swap page to get started."

**CTA:**
- **Primary:** "Make your first swap" → Navigate to `/` (swap page)
- **Secondary:** "Learn about trading on Stellar" (docs link)

**Variant:** `empty`

**Implementation:**
```tsx
<ViewState
  variant="empty"
  title="No transactions yet"
  description="You haven't made any swaps. Head to the Swap page to get started."
  action={
    <Button onClick={() => navigate("/")} variant="default">
      Make your first swap
    </Button>
  }
/>
```

**A11y Requirements:**
- Alt text for transaction icon: "Empty transaction history icon"
- Link to swap page must use proper navigation with `aria-label`

---

### State 2: Loading History

**When:** Transaction list is being fetched from indexer  
**Copy:**
- **Title:** "Loading transactions"
- **Description:** "Fetching your swap history..."

**Variant:** `loading`

**A11y Requirements:**
- Skeleton loader provides visual indication while data loads

---

### State 3: Filters Too Restrictive

**When:** User filters results but no transactions match  
**Copy:**
- **Title:** "No matching transactions"
- **Description:** "Try adjusting your filters or clearing them to see all transactions."

**CTA:**
- **Primary:** "Clear filters" → Reset all filters to defaults
- **Secondary:** None

**Variant:** `empty`

---

### State 4: History Fetch Error

**When:** API fails to fetch transaction history  
**Copy:**
- **Title:** "Could not load history"
- **Description:** "The transaction service is temporarily unavailable. Please try again."

**CTA:**
- **Primary:** "Retry" → Refetch with same filters
- **Secondary:** "Clear filters" → Reset to defaults

**Variant:** `error`

---

## Design Tokens & Styling

### Color Mapping (Tailwind Classes)

All empty states use the existing theme variables:

| Element | Light Theme | Dark Theme | Tailwind Class |
|---------|-------------|-----------|-----------------|
| Icon Background | `muted` | `muted` | `bg-muted` |
| Icon Color | `muted-foreground` | `muted-foreground` | `text-muted-foreground` |
| Error Icon | `destructive` | `destructive` | `text-destructive` |
| Title | `foreground` | `foreground` | `text-foreground` |
| Description | `muted-foreground` | `muted-foreground` | `text-muted-foreground` |
| Border | `border` dashed | `border` dashed | `border-dashed border` |

### Responsive Sizing

- **Mobile (320–640px):** `p-6`, `h-6 w-6` icons
- **Tablet (641–1024px):** `p-8`, `h-6 w-6` icons  
- **Desktop (1024px+):** `p-10`, `h-8 w-8` icons

---

## Implementation Checklist

### Component Updates (`ViewState.tsx`)

- [ ] Add optional `icon` prop to allow custom illustration components
- [ ] Add optional `illustration` prop for custom SVG slots
- [ ] Ensure `role` attributes are properly set (`status` vs `alert`)
- [ ] Verify `aria-hidden="true"` on icons
- [ ] Add optional `headingLevel` for semantic HTML (`h2`, `h3`, etc.)

### Surfaces to Update

#### Swap Page (`/`)
- [ ] Empty pair selection state
- [ ] Quote loading state
- [ ] No liquidity error
- [ ] Network error state

#### Orderbook Page (`/orderbook`)
- [ ] No markets available (syncing)
- [ ] Loading markets
- [ ] No orderbook entries for pair
- [ ] Orderbook fetch error

#### History Page (`/history`)
- [ ] No transactions (fresh wallet)
- [ ] Loading transactions
- [ ] Filters too restrictive
- [ ] History fetch error

### Accessibility Audit
- [ ] All empty states tested with screen reader (NVDA/JAWS/VoiceOver)
- [ ] All CTAs keyboard accessible (Tab, Enter)
- [ ] All states meet WCAG 2.1 AA color contrast (see #462)
- [ ] Icon intent never relies on color alone

### Testing
- [ ] Unit tests for each empty state copy variation
- [ ] E2E tests triggering each state (mock API responses)
- [ ] Mobile responsive tests (320px, 375px, 768px viewports)
- [ ] Dark mode screenshots

---

## Future Enhancements

1. **Custom Illustrations:** Add SVG illustration slots for brand consistency
2. **Animated Empty States:** Subtle animations to guide user attention
3. **Recovery Suggestions:** ML-based suggestions for next steps (e.g., "Try XLM/USDC instead")
4. **Multi-language Support:** Localized copy for all states (see `frontend/lib/formatting.ts`)

---

## Design Review Sign-Off

**Issue:** #461  
**Review Date:** [TO BE FILLED BY DESIGN TEAM]  
**Reviewer:** [TO BE FILLED]  
**Status:** ⬜ Pending → ✅ Approved  

**Sign-Off Comment:** 

---

## Related Issues & Documentation

- **#462:** Accessibility color contrast audit (ensure token contrast ratios)
- **#463:** Information architecture (ensures empty states fit navigation flows)
- [ViewState Component](../../frontend/components/shared/ViewState.tsx)
- [WCAG 2.1 AA Color Contrast Guidelines](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum)

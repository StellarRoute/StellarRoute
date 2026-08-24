# Orderbook Row Highlighting Feature

## Overview

This feature implements synchronized highlighting between the swap panel and the orderbook page. When a user selects a trading pair in the swap panel, the corresponding orderbook view is visually highlighted to provide better context and user experience.

## Implementation Details

### Components Modified

#### 1. TradingPairContext (`frontend/contexts/TradingPairContext.tsx`)

A new React context that manages the currently selected trading pair across the application.

**Features:**
- Stores `fromAsset` and `toAsset` state
- Provides `setTradingPair()` and `clearTradingPair()` methods
- Offers two hooks:
  - `useTradingPair()`: Throws error if used outside provider (strict)
  - `useOptionalTradingPair()`: Returns undefined if used outside provider (optional)

**Usage:**
```typescript
const tradingPairContext = useOptionalTradingPair();
tradingPairContext?.setTradingPair('native', 'USDC:...');
```

#### 2. SwapCard (`frontend/components/swap/SwapCard.tsx`)

Updated to broadcast the current trading pair to the context whenever tokens change.

**Changes:**
- Added `useOptionalTradingPair()` hook
- Added `useEffect` to update context when `fromToken` or `toToken` changes
- Automatically syncs swap panel state with global trading pair context

#### 3. Orderbook Page (`frontend/app/orderbook/page.tsx`)

Enhanced to highlight orderbook rows when they match the trading pair from the swap context.

**Features:**
- Reads trading pair from context using `useOptionalTradingPair()`
- Compares current orderbook pair with context pair (bidirectional matching)
- Applies visual highlighting when pairs match:
  - Animated indicator banner at the top
  - Ring border around bid/ask cards
  - Subtle background tint on individual rows
  - Enhanced hover effects

**Visual Indicators:**
- **Highlighted Pair Banner**: Shows pulsing dot with message "This pair is currently selected in the swap panel"
- **Card Highlighting**: 2px primary-colored ring with shadow
- **Row Highlighting**: Subtle background tint (emerald for bids, red for asks)
- **Hover Effects**: Scale transform and background color change on hover

#### 4. App Providers (`frontend/app/providers.tsx`)

Updated to include `TradingPairProvider` in the provider tree.

**Provider Hierarchy:**
```
NextThemesProvider
  └─ SessionRecoveryProvider
      └─ SettingsProvider
          └─ WalletProvider
              └─ TradingPairProvider  ← New
```

### Styling

The feature uses Tailwind CSS classes for styling:

- **Highlighted indicator**: `bg-primary/10 border-primary/20` with animated pulse
- **Highlighted cards**: `ring-2 ring-primary/30 shadow-lg shadow-primary/10`
- **Highlighted bid rows**: `bg-emerald-500/5` with `hover:bg-emerald-500/10`
- **Highlighted ask rows**: `bg-red-500/5` with `hover:bg-red-500/10`
- **Hover effects**: `hover:scale-[1.02] cursor-pointer transition-all duration-200`

### Bidirectional Matching

The feature supports bidirectional pair matching:
- If swap shows XLM → USDC, it highlights XLM/USDC orderbook
- If swap shows USDC → XLM, it also highlights XLM/USDC orderbook
- This ensures the highlighting works regardless of swap direction

## Testing

### Test Coverage

1. **TradingPairContext Tests** (`frontend/contexts/TradingPairContext.test.tsx`)
   - 9 tests covering provider, hooks, and state management
   - Tests for setting/clearing trading pairs
   - Tests for optional vs required hook behavior

2. **Orderbook Page Tests** (`frontend/app/orderbook/page.test.tsx`)
   - 5 tests covering rendering, loading, error states
   - Tests for hover effects
   - Tests for highlighting behavior

### Running Tests

```bash
# Run all context tests
npm --prefix frontend run test -- contexts/TradingPairContext.test.tsx

# Run orderbook page tests
npm --prefix frontend run test -- app/orderbook/page.test.tsx

# Run all tests
npm --prefix frontend run test
```

## User Experience

### Flow

1. User navigates to swap page
2. User selects trading pair (e.g., XLM/USDC)
3. Context automatically updates with selected pair
4. User navigates to orderbook page
5. If orderbook shows the same pair, visual highlighting appears:
   - Banner indicator at top
   - Glowing border around cards
   - Subtle row background tints
   - Enhanced hover effects

### Accessibility

- **Keyboard Navigation**: All interactive elements are keyboard accessible
- **Screen Readers**: Semantic HTML with proper ARIA labels
- **Color Contrast**: Meets WCAG 2.1 AA standards
- **Focus Indicators**: Visible focus states on all interactive elements
- **Responsive Design**: Works on mobile and desktop viewports

## Mobile Responsiveness

The feature is fully responsive:
- Indicator banner adapts to small screens
- Cards stack vertically on mobile (`md:grid-cols-2`)
- Touch-friendly hover states
- Readable text sizes across all viewports

## Performance Considerations

- **Context Updates**: Minimal re-renders using `useCallback` for context methods
- **Memoization**: `useMemo` for expensive pair matching logic
- **CSS Transitions**: Hardware-accelerated transforms for smooth animations
- **Conditional Rendering**: Highlighting only applied when pairs match

## Future Enhancements

Potential improvements for future iterations:

1. **Deep Linking**: URL parameters to share specific orderbook + swap combinations
2. **Animation**: Smooth transition when highlighting appears/disappears
3. **Customization**: User preferences for highlighting intensity
4. **Multi-Pair**: Support for watching multiple pairs simultaneously
5. **Notifications**: Toast when navigating to highlighted orderbook

## Related Files

- `frontend/contexts/TradingPairContext.tsx` - Context implementation
- `frontend/contexts/TradingPairContext.test.tsx` - Context tests
- `frontend/components/swap/SwapCard.tsx` - Swap panel integration
- `frontend/app/orderbook/page.tsx` - Orderbook highlighting
- `frontend/app/orderbook/page.test.tsx` - Orderbook tests
- `frontend/app/providers.tsx` - Provider setup

## Issue Reference

- **Issue**: #525
- **Milestone**: M4 — Orderbook UI
- **Complexity**: Medium
- **Status**: Completed

## Acceptance Criteria

✅ Implementation meets summary scope  
✅ Tests cover primary user flow  
✅ Accessible on mobile and desktop  
✅ Documented in feature documentation

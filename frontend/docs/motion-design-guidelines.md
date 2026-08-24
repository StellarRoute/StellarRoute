# Motion Design Guidelines

This document outlines the motion design system for StellarRoute, including durations, easing functions, and accessibility considerations.

## Core Principles

1. **Accessibility First**: All animations respect the user's `prefers-reduced-motion` setting
2. **Subtlety**: Animations should enhance, not distract from, the user experience
3. **Consistency**: Use standardized durations and easing across the app
4. **Performance**: Avoid layout-thrashing animations

## Motion Durations

Use these standardized durations for all animations:

| Duration | Value (ms) | Use Case |
|----------|------------|----------|
| `MOTION_DURATION.MICRO` | 150 | Hover states, small UI tweaks, button presses |
| `MOTION_DURATION.QUOTE_REFRESH` | 200 | Subtle updates to quote values |
| `MOTION_DURATION.SKELETON_EXIT` | 260 | Skeleton loader fade-out |
| `MOTION_DURATION.STANDARD` | 300 | Most transitions (modals, panels, etc.) |
| `MOTION_DURATION.ROUTE_SWITCH` | 320 | Switching between different routes |
| `MOTION_DURATION.LONG` | 500 | Complex or multi-step animations |

## Easing Functions

Use these easing functions for consistent motion:

| Easing | Value | Use Case |
|--------|-------|----------|
| `MOTION_EASING.DEFAULT` | `ease-out` | Most animations (objects entering the screen) |
| `MOTION_EASING.IN_OUT` | `ease-in-out` | Bidirectional transitions |
| `MOTION_EASING.BOUNCE` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Playful elements (use sparingly) |

## Accessibility

### Reduced Motion

All animations must respect the `prefers-reduced-motion` media query. This is handled automatically by:

- `useReducedMotion()` hook
- `useMotionValue()` hook
- `useMotionTransition()` hook
- `useMotionAnimateIn()` hook

When reduced motion is enabled:
- All decorative animations are disabled
- Only essential opacity transitions are preserved
- Skeleton loaders use a static dimmed appearance instead of pulsing

### No Motion Blocking Content

Critical content (like quote data) must never be hidden behind an animation. Skeleton loaders should only be used while data is loading, and should be replaced immediately once data is available.

## Usage Examples

### Quote Refresh Animation

Use `useQuoteRefreshTransition()` for quote value updates:

```tsx
import { useQuoteRefreshTransition } from '@/hooks/useQuoteRefreshTransition';

function QuoteSummary({ rate, fee, priceImpact, quoteKey }) {
  const { isRefreshing, transitionStyle } = useQuoteRefreshTransition(quoteKey);
  
  return (
    <div style={transitionStyle} className={isRefreshing ? 'bg-primary/5' : ''}>
      {/* Quote content */}
    </div>
  );
}
```

### Route Switch Animation

Use `useRouteSwitchTransition()` for switching between routes:

```tsx
import { useRouteSwitchTransition } from '@/hooks/useRouteSwitchTransition';

function RouteDisplay({ routeId, ...props }) {
  const { animateInClass } = useRouteSwitchTransition(routeId);
  
  return (
    <div className={animateInClass}>
      {/* Route content */}
    </div>
  );
}
```

### Skeleton Exit Animation

The `useProgressiveLoadingTransition()` hook already uses the motion system:

```tsx
import { useProgressiveLoadingTransition } from '@/hooks/useProgressiveLoadingTransition';

function Component({ isLoading, data }) {
  const { showSkeleton, contentClassName } = useProgressiveLoadingTransition(isLoading);
  
  if (showSkeleton) {
    return <SkeletonLoader />;
  }
  
  return <div className={contentClassName}>{/* Content */}</div>;
}
```

## Swap Module Components

These components already implement the motion design system:
- `QuoteSummary` - Uses `useQuoteRefreshTransition`
- `RouteDisplay` - Uses `useRouteSwitchTransition`
- All components that use `useProgressiveLoadingTransition` - Respect reduced motion

## CSS Considerations

The global styles already include reduced-motion handling in `app/globals.css`. Avoid adding custom animations without checking the reduced-motion setting.

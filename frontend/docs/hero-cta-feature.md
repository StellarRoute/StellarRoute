# Hero CTA Deep Link Feature

## Overview
The landing page hero section now includes a prominent call-to-action (CTA) button that deep links to the swap page with a featured trading pair (XLM/USDC) prefilled.

## Implementation Details

### Components Created

#### 1. HeroSection Component (`components/HeroSection.tsx`)
A new hero section component featuring:
- Eye-catching headline: "Swap Smarter on Stellar"
- Descriptive subheading explaining the aggregator functionality
- Primary CTA button linking to swap page with XLM/USDC pair prefilled
- Secondary CTA button for exploring all pairs
- Feature pills highlighting key benefits (Best Rates, Instant Execution, Secure & Audited)
- Animated background gradients for visual appeal
- Fully responsive design for mobile and desktop

#### 2. Updated Homepage (`app/page.tsx`)
- Integrated the new HeroSection component
- Restructured layout to feature the hero prominently
- Maintained existing DemoSwap functionality below the hero

### Deep Link Implementation

The primary CTA button generates a URL with query parameters:
```
/swap?from=native&to=USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN&amount=100&ts=[timestamp]
```

Parameters:
- `from`: Source token (native = XLM)
- `to`: Destination token (USDC with issuer address)
- `amount`: Prefilled amount (100 XLM)
- `ts`: Timestamp for cache busting and staleness detection

### SwapCard URL Parameter Handling

Enhanced `SwapCard.tsx` to initialize form state from URL parameters:
- Reads URL parameters using `useShareableQuote` hook
- Applies parameters to form state on mount
- Supports prefilling: from token, to token, amount, and slippage
- Includes staleness detection for shared quotes

## Featured Trading Pair

**XLM/USDC** was chosen as the featured pair because:
- High liquidity and trading volume
- Popular trading pair on Stellar
- Good representation of native asset to stablecoin swaps
- Familiar to most Stellar users

## Testing

Comprehensive test suite created (`components/HeroSection.test.tsx`):
- ✅ Renders main heading
- ✅ Renders subheading with key features
- ✅ Primary CTA button with correct deep link
- ✅ Secondary CTA button
- ✅ Feature pills
- ✅ Badge with execution message
- ✅ Proper accessibility structure

All tests passing with proper mocking for Next.js Link and Lucide icons.

## Accessibility

- Semantic HTML structure with proper heading hierarchy
- ARIA-compliant button and link elements
- Keyboard navigation support
- Screen reader friendly content
- Responsive design for all device sizes

## Mobile Responsiveness

The hero section is fully responsive:
- Flexible layout adapts to screen size
- Text sizes scale appropriately (sm:, lg: breakpoints)
- CTA buttons stack vertically on mobile
- Feature pills wrap gracefully
- Touch-friendly button sizes

## Future Enhancements

Potential improvements:
1. Make featured pair configurable via config file
2. Add A/B testing for different featured pairs
3. Rotate featured pairs based on liquidity/volume
4. Add analytics tracking for CTA clicks
5. Implement dynamic amount based on market conditions

## Files Modified/Created

### Created:
- `frontend/components/HeroSection.tsx`
- `frontend/components/HeroSection.test.tsx`
- `frontend/docs/hero-cta-feature.md`

### Modified:
- `frontend/app/page.tsx` - Integrated HeroSection
- `frontend/components/swap/SwapCard.tsx` - Added URL parameter initialization
- `frontend/__mocks__/lucide-react.tsx` - Added missing icon mocks
- `frontend/components/providers/wallet-provider.tsx` - Fixed duplicate imports

## Usage

Users can now:
1. Land on the homepage
2. Click "Start Trading XLM/USDC" button
3. Be taken directly to swap page with pair prefilled
4. Immediately start trading without manual token selection

This streamlines the user journey and reduces friction for new users.

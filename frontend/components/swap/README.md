# Token Pair Selector Component

A comprehensive token pair selector for the Stellar DEX swap flow, allowing users to choose base (sell) and quote (buy) assets.

## Features

- **Asset Selection**: Pick base and quote assets from available trading pairs
- **Swap Sides**: One-click control to flip base and quote assets
- **Search & Filter**: Search by asset code, name, or issuer address
- **Issuer Handling**: Gracefully truncates long issuer strings with copy-to-clipboard functionality
- **Invalid Pair Detection**: Clear messaging when selected pair is not available via API
- **URL State Management**: Selection persists in URL for refresh/back navigation
- **Responsive Design**: Works seamlessly on mobile and desktop
- **Accessibility**: Full keyboard navigation and screen reader support

## Usage

### Basic Example

```tsx
import { TokenPairSelector } from "@/components/swap";
import { usePairs } from "@/hooks/useApi";

function MySwapPage() {
  const { data: pairsData } = usePairs();
  const [base, setBase] = useState<string>();
  const [quote, setQuote] = useState<string>();

  const handlePairChange = (newBase: string, newQuote: string) => {
    setBase(newBase);
    setQuote(newQuote);
  };

  return (
    <TokenPairSelector
      pairs={pairsData?.pairs || []}
      selectedBase={base}
      selectedQuote={quote}
      onPairChange={handlePairChange}
    />
  );
}
```

### With URL State Management

```tsx
import { TokenPairSelector } from "@/components/swap";
import { usePairs } from "@/hooks/useApi";
import { useTokenPairUrl } from "@/hooks/useTokenPairUrl";

function MySwapPage() {
  const { data: pairsData, loading, error } = usePairs();
  const { base, quote, setPair } = useTokenPairUrl();

  return (
    <TokenPairSelector
      pairs={pairsData?.pairs || []}
      selectedBase={base}
      selectedQuote={quote}
      onPairChange={setPair}
      loading={loading}
      error={error ? "Failed to load pairs" : undefined}
    />
  );
}
```

## Props

### TokenPairSelector

| Prop | Type | Required | Description |
|------|------|----------|-------------|
| `pairs` | `TradingPair[]` | Yes | Available trading pairs from the API |
| `selectedBase` | `string` | No | Currently selected base asset (canonical format) |
| `selectedQuote` | `string` | No | Currently selected quote asset (canonical format) |
| `onPairChange` | `(base: string, quote: string) => void` | Yes | Callback when pair selection changes |
| `loading` | `boolean` | No | Loading state for the component |
| `error` | `string` | No | Error message to display |
| `className` | `string` | No | Additional CSS classes |

## Asset Format

Assets use Stellar's canonical format:
- Native XLM: `"native"`
- Issued assets: `"CODE:ISSUER"` (e.g., `"USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"`)

## URL State Hook

The `useTokenPairUrl` hook manages token pair state in URL query parameters:

```tsx
const { base, quote, setPair, isInitializing } = useTokenPairUrl();
```

### Returns

- `base`: Current base asset from URL (`?base=native`)
- `quote`: Current quote asset from URL (`?quote=USDC:ISSUER`)
- `setPair(base, quote)`: Update both assets in URL
- `isInitializing`: Whether URL state is being initialized

## Testing

Run tests with:

```bash
npm test TokenPairSelector
```

## Performance Thresholds

To keep the DOM small on low-end devices, long lists now switch to windowed rendering once they cross these thresholds:

- alternative route list: virtualize when more than `8` items are present
- transaction history activity list: virtualize when more than `24` rows are present

Focused verification for the windowing behavior:

```bash
npm test -- components/swap/RouteDisplay.test.tsx components/TransactionHistory.test.tsx
```

## Design Decisions

1. **Two-Step Selection**: Users select base first, then quote. This ensures only valid pairs can be selected.

2. **Issuer Truncation**: Long issuer addresses are truncated to `XXXXXX...XXXX` format for readability, with full address available via copy button.

3. **Invalid Pair Handling**: When an invalid pair is detected, clear messaging is shown with actionable links to fix the selection.

4. **URL Persistence**: Query parameters ensure the selection survives page refresh and enables shareable links.

5. **Swap Validation**: The swap button is only enabled when the reverse pair exists in the available pairs list.

## Accessibility

- Full keyboard navigation support
- ARIA labels for screen readers
- Focus management in dialogs
- Clear error messaging
- Semantic HTML structure

## Browser Support

Works in all modern browsers that support:
- ES2020+
- CSS Grid
- Flexbox
- Dialog element (polyfilled by Radix UI)

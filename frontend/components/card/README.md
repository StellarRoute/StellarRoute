# Card Components

This directory contains UI components for the StellarRoute card feature, implementing CARD-19, CARD-20, and CARD-21.

## Components

### CardFace (CARD-19)
**Location**: `components/card/CardFace.tsx`

A visual representation of a virtual card showing:
- Last 4 digits of card number
- Expiry date (MM/YY format)
- USDC available balance
- Settlement information message
- Frozen state with visual overlay

**Usage**:
```tsx
import { CardFace } from '@/components/card';

<CardFace
  last4="4567"
  expiry="12/25"
  usdcAvailable="1,234.56"
  isFrozen={false}
/>
```

**Props**:
- `last4` (string): Last 4 digits of card number
- `expiry` (string): Expiration date in MM/YY format
- `usdcAvailable` (string): Available USDC balance (formatted)
- `isFrozen` (boolean, optional): Whether card spending is paused
- `className` (string, optional): Additional CSS classes

**Features**:
- Spacious, card-like layout with gradient background
- Greyscale effect and "Spending Paused" overlay when frozen
- Fully accessible with ARIA labels
- Responsive design with proper aspect ratio
- Dark mode support

---

### ViewFullDetailsButton (CARD-20)
**Location**: `components/card/ViewFullDetailsButton.tsx`

Button that opens the issuer partner's platform for full card details. Emphasizes that StellarRoute never holds keys or card PANs.

**Usage**:
```tsx
import { ViewFullDetailsButton, ViewFullDetailsDisclaimer } from '@/components/card';

<ViewFullDetailsButton
  issuerPartnerName="Acme Bank"
  issuerPartnerUrl="https://partner.example.com/cards"
  variant="primary"
/>

<ViewFullDetailsDisclaimer issuerPartnerName="Acme Bank" />
```

**ViewFullDetailsButton Props**:
- `issuerPartnerUrl` (string): URL to issuer's card details page
- `issuerPartnerName` (string, optional): Display name of issuer (default: "issuing partner")
- `onClick` (function, optional): Callback before opening URL
- `variant` ('primary' | 'secondary' | 'text', optional): Button style variant
- `disabled` (boolean, optional): Disable button
- `className` (string, optional): Additional CSS classes

**ViewFullDetailsDisclaimer Props**:
- `issuerPartnerName` (string, optional): Display name of issuer
- `className` (string, optional): Additional CSS classes

**Features**:
- Opens in new tab with security attributes (noopener, noreferrer)
- Three visual variants (primary, secondary, text)
- Includes eye icon and external link indicator
- Accessible with descriptive ARIA labels
- Disclaimer component explains security model

---

### CardConfirm (CARD-21)
**Location**: `app/card/confirm.tsx`

Pre-signature transaction confirmation view showing all transaction details before wallet signature is requested.

**Usage**:
```tsx
import { CardConfirm } from '@/app/card';

<CardConfirm
  merchant="Coffee Shop"
  fiatAmount="25.00"
  fiatCurrency="USD"
  usdcGross="24.88"
  fee="0.12"
  exchangeRate="1.005"
  onConfirm={handleConfirm}
  onCancel={handleCancel}
  isLoading={false}
/>
```

**Props**:
- `merchant` (string): Merchant/store name
- `fiatAmount` (string): Amount in fiat currency
- `fiatCurrency` (string): Fiat currency code (USD, EUR, etc.)
- `usdcGross` (string): USDC amount before fee
- `fee` (string): Transaction fee in USDC
- `exchangeRate` (string): Exchange rate (1 USDC = X fiat)
- `onConfirm` (function, optional): Callback when confirmed
- `onCancel` (function, optional): Callback when cancelled
- `isLoading` (boolean, optional): Show loading state

**Features**:
- Clear breakdown of all costs before signature
- Automatic total calculation (gross + fee)
- Loading state with spinner
- Cancel and Confirm actions
- Warning about wallet signature requirement
- Accessible button labels
- Dark mode support

---

## Design Principles

### Additive-Only Constraint
All components are **additive-only** and do not modify existing functionality:
- ✅ No changes to swap, quote, or offramp routes
- ✅ No modifications to existing API contracts
- ✅ No changes to wallet connect/sign adapters
- ✅ Isolated in dedicated directories
- ✅ Independent of frozen routes

### Security Model
- StellarRoute **never holds card keys or PANs**
- Full card details (CVV, full number, PIN) are managed by issuing partner
- All components emphasize this in UI/documentation
- External links use secure attributes (noopener, noreferrer)

### Accessibility
All components follow WCAG guidelines:
- Semantic HTML with proper ARIA labels
- Keyboard navigation support
- Screen reader friendly
- Sufficient color contrast
- Focus indicators

### Testing
Each component has comprehensive test coverage:
- Unit tests with Vitest and React Testing Library
- Rendering tests for all states
- Interaction tests for user actions
- Accessibility tests
- Edge case handling
- Dark mode verification

---

## File Structure

```
components/card/
├── CardFace.tsx                    # CARD-19: Card display component
├── CardFace.test.tsx               # Tests for CardFace
├── ViewFullDetailsButton.tsx       # CARD-20: Partner navigation
├── ViewFullDetailsButton.test.tsx  # Tests for ViewFullDetailsButton
├── index.ts                        # Module exports
└── README.md                       # This file

app/card/
├── confirm.tsx                     # CARD-21: Confirmation view
├── confirm.test.tsx                # Tests for CardConfirm
└── index.ts                        # Module exports
```

---

## Running Tests

```bash
# Run all card component tests
npm test -- card

# Run specific component tests
npm test -- CardFace.test.tsx
npm test -- ViewFullDetailsButton.test.tsx
npm test -- confirm.test.tsx

# Run tests in watch mode
npm test -- --watch card
```

---

## Dependencies

These components use:
- React (functional components with TypeScript)
- Tailwind CSS for styling
- Vitest for testing
- React Testing Library for component tests

No additional dependencies required.

---

## Future Enhancements

Potential additions (not in current scope):
- Card transaction history
- Real-time balance updates
- Card freeze/unfreeze toggle
- Multiple card support
- Transaction notifications
- Spending limits display

---

## Contributing

When modifying these components:
1. Maintain additive-only constraint
2. Do not modify frozen functionality
3. Add/update tests for all changes
4. Follow existing code style
5. Update this README if adding new components
6. Ensure accessibility compliance
7. Test in both light and dark modes

---

## Questions?

Refer to the main project documentation or the audit documentation in `/audit/` for security and architecture details.

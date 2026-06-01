# Wallet Connection Onboarding Wireframes

## Overview
This document describes the first-time wallet connection onboarding flow for StellarRoute users. The flow guides new users through the wallet installation and connection process with clear explanations and error handling.

## Flow States

### 1. Welcome Screen
**Purpose**: Introduce the wallet connection flow and explain why it's needed.

**Content**:
- Title: "Connect Your Wallet"
- Description: "Get started with StellarRoute by connecting your Stellar wallet"
- Info box explaining:
  - Supported wallets: Freighter, xBull
  - Why we need wallet connection (display balance, execute trades, manage history)
  - Security assurance: "We never access your private keys. All transactions require your explicit approval."
- Buttons: "Cancel" | "Continue"

**States**:
- Desktop: Modal centered on screen
- Mobile: Full-width dialog, bottom-aligned

---

### 2. Wallet Selection Screen
**Purpose**: Let users choose between installed or install new wallets.

**Content**:
- Title: "Select Your Wallet"
- Description: "Choose which Stellar wallet you'd like to connect"
- Wallet cards for each supported wallet:
  - **If installed**: 
    - Card shows wallet name and "Detected on your device" badge
    - Tap to connect
  - **If not installed**:
    - Card appears with dashed border
    - Shows "Not installed" status
    - External link icon indicating action needed
    - Tap opens wallet installation page in new tab
- Buttons: "Back" | (automatic on wallet selection)

**States**:
- Desktop: Grid layout with wallet cards
- Mobile: Single column, full-width cards

---

### 3. Connection Loading Screen
**Purpose**: Show user the connection is in progress and wait for wallet approval.

**Content**:
- Title: "Connecting [Wallet Name]"
- Description: "Please approve the connection in your wallet"
- Centered loading spinner (animated)
- Message: "Waiting for approval..."
- Help text: "A popup or notification should appear in your wallet. Please review and approve the connection request."

**States**:
- Desktop: Centered modal with loading animation
- Mobile: Full-screen with prominent spinner

---

### 4. Success Screen
**Purpose**: Confirm successful wallet connection.

**Content**:
- Success icon (green checkmark)
- Title: "Wallet Connected!"
- Description: "Your [Wallet Name] wallet is now connected"
- Message: "Connection Successful - You're ready to start trading on StellarRoute"
- Button: "Start Trading" (closes modal and enables main app)

**States**:
- Desktop: Centered modal with success animation
- Mobile: Full-screen celebration state

---

### 5. Error/Failure Screen
**Purpose**: Show connection error and provide recovery options.

**Content**:
- Title: "Connection Failed"
- Description: "We encountered an issue connecting your wallet"
- Error alert showing specific error message
- Troubleshooting tips:
  - Ensure your wallet extension/app is enabled
  - Try refreshing the page
  - Check that you're using the correct network
  - Clear your browser cache and try again
- Buttons: "Try Different Wallet" | "Retry"

**States**:
- Desktop: Modal with error alert and actionable buttons
- Mobile: Full-screen error state with prominent buttons

**Common Error Messages**:
- "Connection request was rejected" → User denied access
- "Wallet is locked" → User needs to unlock wallet
- "Wallet not installed" → User needs to install wallet
- "Failed to get address" → Technical error, retry recommended
- Generic: "Unable to connect wallet. Please try again."

---

### 6. Network Mismatch Screen
**Purpose**: Alert user when wallet is on different network than app.

**Content**:
- Title: "Network Mismatch"
- Description: "Your wallet is on a different network"
- Alert showing:
  - Wallet network vs App network
  - Recommendation to switch wallet to app's network
- Message: "To continue, please switch your wallet to the [app network] network, or you can proceed at your own risk."
- Display: `Wallet: [wallet-network] | App: [app-network]`
- Buttons: "Try Again" | "Proceed Anyway"

**States**:
- Desktop: Warning modal with network comparison
- Mobile: Full-screen warning with prominent call-to-action

---

## Mobile Responsiveness

### Breakpoints
- **Mobile** (< 768px): Full-screen dialogs, single-column layouts
- **Tablet** (768px - 1024px): Medium modal, readable fonts
- **Desktop** (> 1024px): Standard centered modal

### Touch-Friendly Elements
- Minimum button size: 44px × 44px
- Wallet cards: Full-width tappable areas
- Loading spinner: Large, easy to see
- Spacing: Generous padding and gaps

---

## Copy Guidelines for Non-Technical Users

### Plain Language
- ✓ Use "Wallet" instead of "Smart contract" or "Key pair"
- ✓ Use "Connect" instead of "Authorize transaction signing"
- ✓ Use "Approve the connection" instead of "Sign the access request"
- ✓ Use "Your account" instead of "Your public key"

### Error Messages
- ✗ "requestAccess() rejected by user"
- ✓ "You declined the connection. Please try again."
- ✗ "Invalid network passphrase"
- ✓ "Your wallet is on a different network. Please switch to the correct network."

### Help Text
- Always explain WHY we need the wallet connection
- Emphasize security: "Your private keys stay safe in your wallet"
- Provide next steps: "What to expect after connecting"

---

## Implementation Components

### React Components
- `WalletConnectionOnboarding.tsx` - Main modal component with all states
- `useWalletOnboarding.ts` - Hook to manage onboarding state and first-time tracking
- Updated `wallet-button.tsx` - Triggers onboarding for first-time users

### State Management
- Tracks if user is first-time (localStorage)
- Persists onboarding completion status
- Handles error states and recovery

### Responsive Design
- Tailwind CSS with mobile-first approach
- Sheet component for mobile, Dialog for desktop
- Touch-friendly spacing and interactions

---

## Testing Checklist

### Desktop (Chrome, Firefox, Safari)
- [ ] Welcome screen displays correctly
- [ ] Wallet selection shows installed wallets
- [ ] Connection flow handles success
- [ ] Connection flow handles errors
- [ ] Network mismatch is detected and shown
- [ ] All buttons are clickable and functional

### Mobile (iOS Safari, Android Chrome)
- [ ] Full-screen layout is responsive
- [ ] Touch targets are adequately sized
- [ ] Buttons are easy to tap
- [ ] Scrolling works smoothly
- [ ] Loading spinner is visible

### Wallet Integration
- [ ] Freighter integration works
- [ ] xBull integration works
- [ ] Installation links work for non-installed wallets
- [ ] Rejection/error handling works

### First-Time User Experience
- [ ] Onboarding modal shows for first-time users only
- [ ] Onboarding state persists across page refreshes
- [ ] Onboarding can be completed or skipped
- [ ] Returning users don't see onboarding again

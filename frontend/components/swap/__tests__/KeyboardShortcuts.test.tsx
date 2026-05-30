import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SwapCard } from '../SwapCard';

vi.mock('@/hooks/useApi', () => ({
  usePairs: vi.fn(() => ({ data: [], loading: false, error: null })),
  useOrderbook: vi.fn(() => ({ data: null, loading: false, error: null })),
  useQuote: vi.fn(() => ({
    data: null,
    loading: false,
    error: null,
    refresh: vi.fn(),
  })),
  useBatchQuote: vi.fn(() => ({
    data: null,
    loading: false,
    error: null,
    refresh: vi.fn(),
  })),
}));

vi.mock('@/hooks/useSwapState', () => ({
  useSwapState: vi.fn(() => ({
    fromToken: 'native',
    setFromToken: vi.fn(),
    toToken: 'USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    setToToken: vi.fn(),
    fromAmount: '',
    setFromAmount: vi.fn(),
    toAmount: '',
    slippage: 0.5,
    setSlippage: vi.fn(),
    deadline: 20,
    setDeadline: vi.fn(),
    quote: {
      data: null,
      loading: false,
      error: null,
      priceImpact: 0,
      fee: null,
      isStale: false,
      isRecovering: false,
      hasPendingRetry: false,
      pendingRetryRemainingMs: 0,
      expiresAtMs: null,
      ttlSeconds: 30,
      refresh: vi.fn(),
      cancelRetry: vi.fn(),
    },
    switchTokens: vi.fn(),
    formattedRate: '',
    pendingRecovery: null,
    restorePending: vi.fn(),
    discardPending: vi.fn(),
    hasRecoverableState: false,
    snapshotCurrent: vi.fn(() => null),
    reset: vi.fn(),
  })),
}));

vi.mock('@/hooks/useExpertSettings', () => ({
  useExpertSettings: vi.fn(() => ({
    expertMode: false,
    bypassConfirmation: false,
    extendedRouteDetails: false,
    updateExpertMode: vi.fn(),
    updateBypassConfirmation: vi.fn(),
    updateExtendedRouteDetails: vi.fn(),
  })),
}));

vi.mock('@/hooks/useBrowserNotifications', () => ({
  useBrowserNotifications: vi.fn(() => ({
    browserNotifications: false,
    permissionState: 'default',
    isDisabled: false,
    enableNotifications: vi.fn(),
    disableNotifications: vi.fn(),
  })),
}));

vi.mock('@/hooks/useCompactMode', () => ({
  useCompactMode: vi.fn(() => ({
    isCompact: false,
    toggleCompact: vi.fn(),
  })),
}));

vi.mock('@/hooks/useReducedMotion', () => ({
  useReducedMotion: vi.fn(() => false),
}));

vi.mock('@/hooks/useOnlineStatus', () => ({
  useOnlineStatus: vi.fn(() => ({ isOnline: true })),
}));

vi.mock('@/hooks/useQuoteStreamStatus', () => ({
  useQuoteStreamStatus: vi.fn(() => ({
    status: 'connected',
    mode: 'live',
  })),
}));

vi.mock('@/lib/swap-i18n', () => ({
  useSwapI18n: vi.fn(() => ({
    t: (key: string, vars?: Record<string, string | number>) => {
      const translations: Record<string, string> = {
        'swap.card.title': 'Swap',
        'swap.card.poweredBy': 'Powered by StellarRoute Aggregator',
        'swap.shortcuts.title': 'Keyboard shortcuts',
        'swap.shortcuts.openHelp': 'Open shortcut help',
        'swap.shortcuts.closeHelp': 'Close modal',
        'swap.shortcuts.focusPayAmount': 'Focus pay amount',
        'swap.shortcuts.focusReceiveAmount': 'Focus receive amount',
        'swap.shortcuts.refreshQuote': 'Refresh quote',
        'swap.shortcuts.flipPair': 'Flip pay/receive pair',
        'swap.shortcuts.maxAmount': 'Set max pay amount',
        'swap.cta.connectWallet': 'Connect Wallet',
        'swap.card.refreshQuote': 'Refresh quote',
        'swap.pair.youPay': 'You Pay',
        'swap.pair.youReceive': 'You Receive',
        'swap.pair.balance': 'Balance: {amount}',
      };
      let msg = translations[key] || key;
      if (vars) {
        for (const [k, v] of Object.entries(vars)) {
          msg = msg.replace(`{${k}}`, String(v));
        }
      }
      return msg;
    },
  })),
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    loading: vi.fn(),
  },
}));

describe('SwapCard keyboard shortcuts', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('opens shortcut help dialog when ? is pressed', async () => {
    render(<SwapCard />);
    const user = userEvent.setup();
    await user.keyboard('?');
    expect(screen.getByText('Keyboard shortcuts')).toBeInTheDocument();
  });

  it('closes shortcut help dialog when Escape is pressed', async () => {
    render(<SwapCard />);
    const user = userEvent.setup();
    await user.keyboard('?');
    expect(screen.getByText('Keyboard shortcuts')).toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(screen.queryByText('Keyboard shortcuts')).not.toBeInTheDocument();
  });

  it('does not open shortcut help when focus is in an input', async () => {
    render(<SwapCard />);
    const user = userEvent.setup();
    const inputs = screen.getAllByRole('textbox');
    if (inputs.length > 0) {
      await user.click(inputs[0]);
      await user.keyboard('?');
      expect(screen.queryByText('Keyboard shortcuts')).not.toBeInTheDocument();
    }
  });

  it('renders the swap card without crashing', () => {
    render(<SwapCard />);
    expect(screen.getByTestId('swap-card')).toBeInTheDocument();
  });
});

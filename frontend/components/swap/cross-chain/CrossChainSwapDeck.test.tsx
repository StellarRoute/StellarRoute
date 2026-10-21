import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { CrossChainSwapDeck } from './CrossChainSwapDeck';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { WalletProvider } from '@/components/providers/wallet-provider';
import { useApiV2Readiness } from '@/hooks/useApiV2Readiness';
import { useCctpSaga } from '@/hooks/useCctpSaga';
import { useCrossChainWalletRoles } from '@/hooks/useCrossChainWalletRoles';
import type { UseCrossChainWalletRolesInput } from '@/hooks/useCrossChainWalletRoles';

vi.mock('sonner', () => ({
  toast: {
    message: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock('next/dynamic', () => ({
  default: () => {
    const MockSwapCard = () => (
      <div data-testid="swap-card">Delegated SwapCard</div>
    );
    return MockSwapCard;
  },
}));

vi.mock('@/hooks/useFeatureFlag', () => ({
  useFeatureFlag: vi.fn(() => ({ enabled: false, loading: false })),
}));

vi.mock('@/hooks/useApiV2Readiness', () => ({
  useApiV2Readiness: vi.fn(() => ({
    loaded: true,
    corridors: [],
    cctpGloballyReady: false,
    providerKilled: false,
    error: null,
    fetchedAt: Date.now(),
    loading: false,
    refresh: vi.fn(),
  })),
}));

vi.mock('@/hooks/useCrossChainWalletRoles', () => ({
  useCrossChainWalletRoles: vi.fn(
    (_input: UseCrossChainWalletRolesInput) => ({
      direction: null,
      destRecipientAddress: '',
      isMuxedRecipient: false,
      showMintSubmitterChip: false,
      sourceChipBinding: null,
      destChipBinding: null,
      mintSubmitterChipBinding: null,
      sagaWallets: { recipient: '' },
    }),
  ),
}));

vi.mock('@/hooks/useCctpSaga', () => ({
  useCctpSaga: vi.fn(() => ({
    stage: 'idle',
    quote: null,
    transferStatus: null,
    error: null,
    busy: false,
    inputsLocked: false,
    resumeMismatch: false,
    sessionPublic: null,
    primaryAction: { label: 'Get quote', disabled: false, action: 'quote' },
    runPrimaryAction: vi.fn(),
    requestQuote: vi.fn(),
    reconcileOnLoad: vi.fn(),
    resetSaga: vi.fn(),
    reattestCooldownUntil: null,
  })),
}));

vi.mock('@/hooks/useChainWallet', () => ({
  useChainWallet: vi.fn(() => ({
    session: null,
    isConnected: false,
    networkMismatch: false,
    isLoading: false,
    availableWallets: [],
    connect: vi.fn(),
    disconnect: vi.fn(),
  })),
}));

import type { CrossChainDeckStoryPresentation } from './crossChainStoryPresentation';

const mockUseApiV2Readiness = vi.mocked(useApiV2Readiness);
const mockUseCctpSaga = vi.mocked(useCctpSaga);
const mockUseCrossChainWalletRoles = vi.mocked(useCrossChainWalletRoles);

function mockStellarToSepoliaWalletRoles(
  _input: UseCrossChainWalletRolesInput,
  destRecipientAddress = '',
) {
  return {
    direction: 'stellar_to_evm' as const,
    destRecipientAddress,
    isMuxedRecipient: false,
    showMintSubmitterChip: false,
    sourceChipBinding: null,
    destChipBinding: null,
    mintSubmitterChipBinding: null,
    sagaWallets: { recipient: destRecipientAddress },
  };
}

/**
 * Mirrors what the real `useCctpSaga` returns when `bridgeReady` is false
 * (`CCTP_ENABLED=false`, or no executable corridor in `/api/v2`). Kept in one
 * place so the gated tests below assert the deck's wiring, not a literal.
 */
function mockGatedCctpSaga() {
  const requestQuote = vi.fn();
  const runPrimaryAction = vi.fn();

  mockUseCctpSaga.mockReturnValue({
    stage: 'idle',
    quote: null,
    transferStatus: null,
    error: null,
    busy: false,
    inputsLocked: false,
    resumeMismatch: false,
    sessionPublic: null,
    primaryAction: { label: 'Bridge unavailable', disabled: true, action: 'none' },
    runPrimaryAction,
    requestQuote,
    reconcileOnLoad: vi.fn(),
    resetSaga: vi.fn(),
    reattestCooldownUntil: null,
  } as unknown as ReturnType<typeof useCctpSaga>);

  return { requestQuote, runPrimaryAction };
}

function renderDeck(presentation?: CrossChainDeckStoryPresentation) {
  return render(
    <SettingsProvider>
      <WalletProvider>
        <CrossChainSwapDeck storyPresentation={presentation} />
      </WalletProvider>
    </SettingsProvider>
  );
}

describe('CrossChainSwapDeck', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('hides From/To selectors and delegates SwapCard on stellar-native', () => {
    renderDeck({
      initialSourceChainId: 'stellar',
      initialDestChainId: 'stellar',
    });
    expect(screen.queryByTestId('paired-chain-selectors')).not.toBeInTheDocument();
    expect(screen.getByTestId('stellar-native-delegation')).toBeInTheDocument();
    expect(screen.getByTestId('swap-card')).toBeInTheDocument();
    expect(screen.getByText(/Bridge USDC/i)).toBeInTheDocument();
  });

  it('defaults to the proven Stellar to Sepolia corridor without aside clutter', () => {
    renderDeck();

    expect(screen.getByTestId('chain-option-source-stellar')).toBeChecked();
    expect(
      screen.getByTestId('chain-option-destination-ethereum-sepolia')
    ).toBeChecked();
    expect(screen.queryByTestId('swap-card')).not.toBeInTheDocument();
    expect(screen.queryByTestId('unsupported-corridor-alert')).not.toBeInTheDocument();
    expect(screen.queryByTestId('cctp-route-rail')).not.toBeInTheDocument();
    expect(screen.queryByTestId('execution-timeline')).not.toBeInTheDocument();
    expect(screen.queryByTestId('destination-recipient-input')).not.toBeInTheDocument();
  });

  it('treats Sepolia to Stellar as a catalog-executable CCTP corridor', () => {
    renderDeck({
      initialSourceChainId: 'ethereum-sepolia',
      initialDestChainId: 'stellar',
    });
    expect(screen.queryByTestId('unsupported-corridor-alert')).not.toBeInTheDocument();
    expect(screen.queryByTestId('swap-card')).not.toBeInTheDocument();
    expect(screen.queryByTestId('cctp-route-rail')).not.toBeInTheDocument();
  });

  it('shows unsupported alert for catalogued coming-soon corridor', () => {
    renderDeck({
      initialSourceChainId: 'solana',
      initialDestChainId: 'stellar',
    });
    expect(screen.getByTestId('unsupported-corridor-alert')).toBeInTheDocument();
    expect(screen.queryByTestId('swap-card')).not.toBeInTheDocument();
  });

  it('blocks uncatalogued Sepolia to Bitcoin with unsupported alert and no CTA', async () => {
    const user = userEvent.setup();
    renderDeck();

    await user.click(screen.getByTestId('chain-option-source-ethereum-sepolia'));
    await user.click(screen.getByTestId('chain-option-destination-bitcoin'));

    expect(screen.getByTestId('unsupported-corridor-alert')).toBeInTheDocument();
    expect(screen.queryByTestId('cross-chain-review-cta')).not.toBeInTheDocument();
    expect(screen.queryByTestId('cctp-route-rail')).not.toBeInTheDocument();
    expect(screen.queryByText(/99\./)).not.toBeInTheDocument();
  });

  it('does not render destination recipient override', () => {
    renderDeck({
      initialSourceChainId: 'ethereum-sepolia',
      initialDestChainId: 'stellar',
    });
    expect(
      screen.queryByLabelText('Use custom destination recipient'),
    ).not.toBeInTheDocument();
    expect(screen.queryByTestId('destination-recipient-input')).not.toBeInTheDocument();
  });

  it('does not render cross-chain review CTA without wallet direction', () => {
    renderDeck({
      initialSourceChainId: 'ethereum-sepolia',
      initialDestChainId: 'stellar',
    });
    expect(screen.queryByTestId('cross-chain-review-cta')).not.toBeInTheDocument();
  });
});

describe('CrossChainSwapDeck CCTP CTA hints', () => {
  beforeEach(() => {
    mockUseApiV2Readiness.mockReturnValue({
      loaded: true,
      corridors: [],
      cctpGloballyReady: true,
      providerKilled: false,
      error: null,
      fetchedAt: Date.now(),
      loading: false,
      refresh: vi.fn(),
    });
    mockUseCrossChainWalletRoles.mockImplementation((input) =>
      mockStellarToSepoliaWalletRoles(input),
    );
  });

  it('shows connect-wallet hint when destination wallet is disconnected', () => {
    renderDeck();

    expect(screen.getByTestId('cross-chain-review-cta')).toBeDisabled();
    expect(screen.getByTestId('cctp-cta-hint')).toHaveTextContent(
      /Connect your ETH Sepolia wallet/i,
    );
    expect(screen.getByTestId('dest-wallet-setup-hint')).toHaveTextContent(
      /Connect your ETH Sepolia wallet/i,
    );
  });

  it('enables CTA when destination wallet is connected and amount is set', async () => {
    const user = userEvent.setup();
    mockUseCrossChainWalletRoles.mockImplementation((input) =>
      mockStellarToSepoliaWalletRoles(
        input,
        '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
      ),
    );
    renderDeck();

    await user.type(screen.getByTestId('cctp-source-amount'), '10');

    expect(screen.queryByTestId('cctp-cta-hint')).not.toBeInTheDocument();
    expect(screen.queryByTestId('dest-wallet-setup-hint')).not.toBeInTheDocument();
    expect(screen.getByTestId('cross-chain-review-cta')).toBeEnabled();
  });

  it('surfaces USDC-only guidance with swap link on CCTP corridor', () => {
    renderDeck();

    expect(screen.getByTestId('cctp-usdc-only-note')).toHaveTextContent(
      /Bridges USDC only/i,
    );
    expect(screen.getByTestId('swap-to-usdc-on-stellar-link')).toBeInTheDocument();
  });
});

describe('CrossChainSwapDeck gated when CCTP is not enabled', () => {
  // With `CCTP_ENABLED=false` the API still answers `/api/v2` with
  // `executable: false` corridors, so the deck must render in a fail-closed
  // state: gated copy, no executable burn/mint control.
  const gatedReadiness = (loading = false) =>
    mockUseApiV2Readiness.mockReturnValue({
      loaded: !loading,
      corridors: [],
      cctpGloballyReady: false,
      providerKilled: false,
      error: null,
      fetchedAt: Date.now(),
      loading,
      refresh: vi.fn(),
    });

  beforeEach(() => {
    vi.clearAllMocks();
    gatedReadiness();
    mockGatedCctpSaga();
    mockUseCrossChainWalletRoles.mockImplementation((input) =>
      mockStellarToSepoliaWalletRoles(
        input,
        '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
      ),
    );
  });

  it('shows the bridge-unavailable copy and no executable burn/mint CTA', async () => {
    const user = userEvent.setup();
    renderDeck();

    // A fully-connected, fully-populated deck is the worst case: every input
    // the live CTA needs is present, and it must still be gated.
    await user.type(screen.getByTestId('cctp-source-amount'), '10');

    const cta = screen.getByTestId('cross-chain-review-cta');
    expect(cta).toBeDisabled();
    expect(cta).toHaveTextContent(/Bridge unavailable/i);
    // The panel and the CTA hint both carry the gated message.
    expect(
      screen.getAllByText(/Bridge is not available on this API right now\./i),
    ).not.toHaveLength(0);
  });

  it('enables no button anywhere in the deck while gated', async () => {
    const user = userEvent.setup();
    renderDeck();

    await user.type(screen.getByTestId('cctp-source-amount'), '10');

    // The acceptance criterion: no burn or mint button may be enabled. Scoped
    // to the execution panel because corridor/chain selectors stay
    // interactive on purpose — switching corridors is not an execution.
    const panel = screen.getByTestId('cctp-execution-panel');
    const panelButtons = panel.querySelectorAll('button');
    expect(panelButtons.length).toBeGreaterThan(0);
    for (const button of panelButtons) {
      expect(button).toBeDisabled();
    }
  });

  it('never labels a control with an executable burn or mint action', async () => {
    const user = userEvent.setup();
    renderDeck();

    await user.type(screen.getByTestId('cctp-source-amount'), '10');

    const executable = /^(Confirm lock on source chain|Confirm receive on destination|Prepare source transaction|Approve USDC spend|Get quote)$/;
    for (const button of screen
      .getByTestId('cctp-execution-panel')
      .querySelectorAll('button')) {
      expect(button.textContent ?? '').not.toMatch(executable);
    }
  });

  it('requests no quote while gated', async () => {
    const user = userEvent.setup();
    const { requestQuote, runPrimaryAction } = mockGatedCctpSaga();
    renderDeck();

    await user.type(screen.getByTestId('cctp-source-amount'), '10');
    await user.click(screen.getByTestId('cross-chain-review-cta'));

    expect(requestQuote).not.toHaveBeenCalled();
    expect(runPrimaryAction).not.toHaveBeenCalled();
  });

  it('prefers the bridge hint over a wallet or amount prompt', async () => {
    renderDeck();

    expect(screen.getByTestId('cctp-cta-hint')).toHaveTextContent(
      /Bridge is not available on this API right now\./i,
    );
  });

  it('stays gated while readiness is still loading', () => {
    gatedReadiness(true);
    renderDeck();

    expect(screen.getByTestId('cross-chain-review-cta')).toBeDisabled();
    expect(screen.getByTestId('cctp-cta-hint')).toHaveTextContent(
      /Checking bridge availability/i,
    );
  });

  it('shows no burn/mint labels, quote or destination amount', () => {
    renderDeck();

    // "Burn"/"Mint" step chips are preview-only rail text, not CTAs; assert no
    // destination amount or execution timeline is surfaced while gated.
    expect(screen.queryByTestId('execution-timeline')).not.toBeInTheDocument();
    expect(screen.queryByText(/0\.00 USDC/)).not.toBeInTheDocument();
  });
});

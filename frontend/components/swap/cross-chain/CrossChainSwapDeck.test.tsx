import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { CrossChainSwapDeck } from './CrossChainSwapDeck';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { WalletProvider } from '@/components/providers/wallet-provider';
import { useApiV2Readiness } from '@/hooks/useApiV2Readiness';
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

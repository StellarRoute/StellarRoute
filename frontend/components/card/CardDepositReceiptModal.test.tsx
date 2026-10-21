import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CardDepositReceiptModal } from './CardDepositReceiptModal';
import type { DepositSagaState, DepositReceipt } from '@/lib/card/receipt';

// ── Mocks ──────────────────────────────────────────────────────────────────────

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

if (typeof global.ResizeObserver === 'undefined') {
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

// ── Fixtures ───────────────────────────────────────────────────────────────────

const BURN_HASH =
  '0x339b96ccb6c3bcc0eb4c37d70fb5b8e6f3ee4c6fd1e7c032e93827faab6a5e73';
const MINT_HASH =
  '13d2025db39b461756954e1266864ea39c126cada55ddf24db9ec364138d16f2';

const fullSagaState: DepositSagaState = {
  sourceChain: 'eip155:11155111',
  source_tx_hash: BURN_HASH,
  destination_tx_hash: MINT_HASH,
  amount: '5',
  timestamp: 1723680000, // Unix seconds
  recipient: '0xa632DA1E4D5DD7FB236a0b798ff9331e3a9930Df',
  status: 'completed',
};

const prebuiltReceipt: DepositReceipt = {
  sourceChain: 'Ethereum Sepolia',
  sourceNetworkBadge: 'Ethereum Sepolia',
  burnTxHash: BURN_HASH,
  mintTxHash: MINT_HASH,
  amount: '5',
  usdcAmount: '5 USDC',
  timestamp: '2024-08-15T04:00:00.000Z',
  timestampFormatted: 'Thu, 15 Aug 2024 04:00:00 GMT',
  timestampMs: 1723694400000,
  recipient: '0xa632DA1E4D5DD7FB236a0b798ff9331e3a9930Df',
  status: 'completed',
  sourceExplorerUrl: `https://sepolia.etherscan.io/tx/${encodeURIComponent(BURN_HASH)}`,
  destinationExplorerUrl: `https://stellar.expert/explorer/testnet/tx/${encodeURIComponent(MINT_HASH)}`,
  markdownSummary: '# Cross-Chain Top-Up Proof of Deposit\n- **Status**: completed',
  textSummary: '=== Cross-Chain Top-Up Proof of Deposit ===\nStatus: completed',
};

// ── Tests ──────────────────────────────────────────────────────────────────────

describe('CardDepositReceiptModal', () => {
  const onOpenChange = vi.fn();
  const onDone = vi.fn();

  let clipboardWriteText: ReturnType<typeof vi.fn>;
  let originalClipboard: PropertyDescriptor | undefined;
  let originalPrint: typeof window.print;

  beforeEach(() => {
    onOpenChange.mockClear();
    onDone.mockClear();

    clipboardWriteText = vi.fn().mockResolvedValue(undefined);
    originalClipboard = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: clipboardWriteText },
      configurable: true,
      writable: true,
    });

    originalPrint = window.print;
    window.print = vi.fn();
  });

  afterEach(() => {
    if (originalClipboard) {
      Object.defineProperty(navigator, 'clipboard', originalClipboard);
    }
    window.print = originalPrint;
    vi.clearAllMocks();
  });

  // ── Metadata extraction ────────────────────────────────────────────────────

  describe('metadata extraction from sagaState', () => {
    it('renders the modal heading', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByText('Cross-Chain Deposit Proof')).toBeInTheDocument();
    });

    it('displays bridged amount from sagaState', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByText('5 USDC')).toBeInTheDocument();
    });

    it('displays source network badge', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByText('Ethereum Sepolia')).toBeInTheDocument();
    });

    it('displays destination network badge (Stellar Testnet)', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByText('Stellar Testnet')).toBeInTheDocument();
    });

    it('displays status', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByText('completed')).toBeInTheDocument();
    });

    it('renders the deposit timestamp element', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByTestId('receipt-timestamp')).toBeInTheDocument();
    });

    it('renders burn tx hash (shortened)', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      // Shortened display – at minimum the first 10 chars should appear somewhere
      expect(screen.getByText(/0x339b96ccb6/)).toBeInTheDocument();
    });

    it('renders mint tx hash (shortened)', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByText(/13d2025db3/)).toBeInTheDocument();
    });

    it('renders explorer hyperlinks for both hashes', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      const links = screen.getAllByRole('link');
      const hrefs = links.map((l) => l.getAttribute('href') ?? '');
      expect(hrefs.some((h) => h.includes('sepolia.etherscan.io'))).toBe(true);
      expect(hrefs.some((h) => h.includes('stellar.expert'))).toBe(true);
    });

    it('renders recipient address when present', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      // Shortened form of the address should appear
      expect(screen.getByText(/0xa632DA/)).toBeInTheDocument();
    });
  });

  // ── Missing hash states ─────────────────────────────────────────────────────

  describe('missing hash fallback states', () => {
    it('shows no-burn-hash placeholder when burn tx is absent', () => {
      const partial: DepositSagaState = { amount: '10', status: 'burn_submitted' };
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={partial}
        />,
      );

      expect(screen.getByTestId('receipt-no-burn-hash')).toBeInTheDocument();
    });

    it('shows no-mint-hash placeholder when mint tx is absent', () => {
      const partial: DepositSagaState = {
        source_tx_hash: BURN_HASH,
        amount: '10',
        status: 'awaiting_attestation',
      };
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={partial}
        />,
      );

      expect(screen.getByTestId('receipt-no-mint-hash')).toBeInTheDocument();
    });
  });

  // ── Pre-built receipt prop ─────────────────────────────────────────────────

  describe('pre-built receipt prop', () => {
    it('uses the receipt prop directly without calling formatDepositReceipt', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          receipt={prebuiltReceipt}
        />,
      );

      expect(screen.getByText('5 USDC')).toBeInTheDocument();
      expect(screen.getByText('Thu, 15 Aug 2024 04:00:00 GMT')).toBeInTheDocument();
    });
  });

  // ── Clipboard copy handler ─────────────────────────────────────────────────

  describe('"Copy Receipt Data" button', () => {
    it('renders the Copy Receipt Data button', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByTestId('card-deposit-receipt-copy')).toBeInTheDocument();
    });

    it('calls clipboard.writeText with markdownSummary when clicked', async () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          receipt={prebuiltReceipt}
        />,
      );

      const copyBtn = screen.getByTestId('card-deposit-receipt-copy');
      fireEvent.click(copyBtn);

      // Allow the microtask queue to flush
      await Promise.resolve();

      expect(clipboardWriteText).toHaveBeenCalledWith(prebuiltReceipt.markdownSummary);
    });
  });

  // ── Print button ───────────────────────────────────────────────────────────

  describe('Print / Save PDF button', () => {
    it('renders the print button', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByTestId('card-deposit-receipt-print')).toBeInTheDocument();
    });

    it('calls window.print when print button is clicked', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      fireEvent.click(screen.getByTestId('card-deposit-receipt-print'));
      expect(window.print).toHaveBeenCalledTimes(1);
    });
  });

  // ── Modal open / close accessibility ──────────────────────────────────────

  describe('modal open/close accessibility', () => {
    it('renders modal content when open=true', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.getByTestId('card-deposit-receipt-modal')).toBeInTheDocument();
    });

    it('does not render modal content when open=false', () => {
      render(
        <CardDepositReceiptModal
          open={false}
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.queryByTestId('card-deposit-receipt-modal')).not.toBeInTheDocument();
    });

    it('calls onOpenChange(false) when Done button is clicked', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          onDone={onDone}
          sagaState={fullSagaState}
        />,
      );

      fireEvent.click(screen.getByTestId('card-deposit-receipt-done'));
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });

    it('calls onDone callback when Done button is clicked', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          onDone={onDone}
          sagaState={fullSagaState}
        />,
      );

      fireEvent.click(screen.getByTestId('card-deposit-receipt-done'));
      expect(onDone).toHaveBeenCalledTimes(1);
    });

    it('calls onOpenChange(false) when "Keep receipt on screen" is clicked', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      fireEvent.click(screen.getByTestId('card-deposit-receipt-keep-open'));
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });

    it('does not call onDone when "Keep receipt on screen" is clicked', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          onDone={onDone}
          sagaState={fullSagaState}
        />,
      );

      fireEvent.click(screen.getByTestId('card-deposit-receipt-keep-open'));
      expect(onDone).not.toHaveBeenCalled();
    });
  });

  // ── Additive constraint ────────────────────────────────────────────────────

  describe('additive constraint: frozen surfaces unchanged', () => {
    it('does not render any swap/offramp/cross-chain-swap content', () => {
      render(
        <CardDepositReceiptModal
          open
          onOpenChange={onOpenChange}
          sagaState={fullSagaState}
        />,
      );

      expect(screen.queryByText(/swap/i)).not.toBeInTheDocument();
      expect(screen.queryByText(/offramp/i)).not.toBeInTheDocument();
    });
  });
});

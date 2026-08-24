import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CctpExecutionPanel } from './CctpExecutionPanel';
import type { CctpTransferStatusResponse } from '@/lib/cctp/types';

const completedStatus: CctpTransferStatusResponse = {
  transfer_id: 'cctp-de979072-e5fb-435d-a35c-c12648ccaa41',
  corridor_id: 'circle-cctp:usdc:stellar-testnet:ethereum-sepolia',
  provider: 'circle-cctp',
  direction: 'stellar_to_evm',
  status: 'completed',
  retryable: false,
  support_reference_id: 'cctp-de979072-e5fb-435d-a35c-c12648ccaa41',
  source_tx_hash: 'sourcehash',
  destination_tx_hash: 'desthash',
};

describe('CctpExecutionPanel', () => {
  it('shows journey visual and replaces Waiting CTA after complete', async () => {
    const user = userEvent.setup();
    const onCompleteDone = vi.fn();
    const onViewReceipt = vi.fn();
    render(
      <CctpExecutionPanel
        stage="completed"
        quote={{
          transfer_id: completedStatus.transfer_id,
          corridor_id: completedStatus.corridor_id,
          provider: 'circle-cctp',
          direction: 'stellar_to_evm',
          source_amount: '25',
          destination_amount: '25',
          fee_quote: {},
          expires_at: Math.floor(Date.now() / 1000) + 600,
          finality: 'standard',
          access_token: 'token',
        }}
        transferStatus={completedStatus}
        error={null}
        primaryLabel="Waiting…"
        primaryDisabled
        onPrimary={vi.fn()}
        onReset={vi.fn()}
        onCompleteDone={onCompleteDone}
        onViewReceipt={onViewReceipt}
        recipient="0xa632da1234567890abcdef1234567890abcdef12"
      />,
    );

    expect(screen.getByTestId('cctp-journey-visual')).toHaveAttribute(
      'data-status',
      'completed',
    );
    expect(screen.getByTestId('cctp-transfer-receipt')).toBeInTheDocument();
    expect(screen.getByText(/Destination \(EVM\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Destination mint tx/i)).toBeInTheDocument();
    expect(screen.getByTestId('cctp-saga-status')).toHaveTextContent(/Complete/i);
    expect(screen.getByText(/Speed/i)).toBeInTheDocument();
    expect(screen.getByText(/Standard timing/i)).toBeInTheDocument();
    const cta = screen.getByTestId('cross-chain-review-cta');
    expect(cta).toHaveTextContent(/Done — start new transfer/i);
    expect(cta).not.toBeDisabled();
    expect(screen.queryByText('Waiting…')).not.toBeInTheDocument();

    await user.click(cta);
    expect(onCompleteDone).toHaveBeenCalledTimes(1);
    await user.click(screen.getByTestId('cctp-view-receipt'));
    expect(onViewReceipt).toHaveBeenCalledTimes(1);
  });

  it('renders journey while awaiting confirmation', () => {
    render(
      <CctpExecutionPanel
        stage="awaiting_attestation"
        quote={null}
        transferStatus={{
          ...completedStatus,
          status: 'awaiting_attestation',
          destination_tx_hash: undefined,
        }}
        error={null}
        primaryLabel="Waiting for confirmation…"
        primaryDisabled
        onPrimary={vi.fn()}
      />,
    );

    expect(screen.getByTestId('cctp-journey-visual')).toHaveAttribute(
      'data-status',
      'awaiting_attestation',
    );
    expect(screen.getByText(/In transit/i)).toBeInTheDocument();
    expect(screen.getByText('Lock')).toBeInTheDocument();
    expect(screen.getByText('Receive')).toBeInTheDocument();
  });

  it('shows Your turn banner and emphasizes CTA when needsUserAction', () => {
    render(
      <CctpExecutionPanel
        stage="sign_mint"
        quote={null}
        transferStatus={{
          ...completedStatus,
          status: 'mint_prepared',
          destination_tx_hash: undefined,
        }}
        error={null}
        primaryLabel="Confirm receive on destination"
        primaryDisabled={false}
        needsUserAction
        nextActionNotice="Your turn — confirm receive on the destination chain."
        onPrimary={vi.fn()}
      />,
    );

    expect(screen.getByTestId('cctp-next-action-banner')).toHaveTextContent(
      /Your turn — confirm receive/i,
    );
    expect(screen.getByTestId('cross-chain-review-cta').className).toMatch(
      /animate-pulse/,
    );
  });
});

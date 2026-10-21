import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi, beforeEach } from 'vitest';

const mockUseSearchParams = vi.hoisted(() => vi.fn(() => new URLSearchParams()));

vi.mock('next/navigation', () => ({
  useSearchParams: () => mockUseSearchParams(),
}));

import {
  isNigerianCbnBankCode,
  isValidNigerianInstitutionCode,
  NIGERIAN_INSTITUTION_CODE_LENGTH,
} from '@/lib/offramp/institution-codes';
import { findOfframpSource } from '@/lib/offramp/assets';
import { NIGERIAN_BANKS } from '@/lib/offramp/nigerian-banks';
import { buildOfframpQuotePreview } from '@/lib/offramp/quote';

import OfframpPage, { metadata } from './page';

/** Paycrest-style 8-character institution code fixture. */
const INSTITUTION_CODE_FIXTURE = '05800001';
/** Legacy 3-digit CBN bank code — not a valid institution code. */
const CBN_BANK_CODE_FIXTURE = '058';

/**
 * NGN quote fixture, built by the same pure helper the page uses so the
 * expected numbers cannot drift from the implementation.
 */
const NGN_QUOTE_FIXTURE = buildOfframpQuotePreview({
  asset: findOfframpSource('stellar-usdc')!,
  amount: '100',
  mode: 'direct',
});

describe('OfframpPage metadata', () => {
  beforeEach(() => {
    mockUseSearchParams.mockReturnValue(new URLSearchParams());
  });

  it('keeps the SEO title, description and canonical path', () => {
    expect(metadata).toMatchObject({
      title: 'Offramp to Naira',
      description: expect.stringContaining('Nigerian Naira (NGN)'),
      alternates: { canonical: expect.stringContaining('/offramp') },
    });
  });

  it('prefills the form when amount and source query params are present', () => {
    mockUseSearchParams.mockReturnValue(new URLSearchParams('amount=250&source=base-usdc'));

    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-amount')).toHaveValue('250');
    expect(screen.getByTestId('offramp-asset-base-usdc')).toHaveAttribute('aria-selected', 'true');
  });

  it('leaves the bare offramp screen unchanged when no query params are present', () => {
    mockUseSearchParams.mockReturnValue(new URLSearchParams());

    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-amount')).toHaveValue('');
    expect(screen.getByTestId('offramp-asset-stellar-usdc')).toHaveAttribute('aria-selected', 'true');
  });
});

describe('OfframpPage empty states', () => {
  it('renders the offramp dashboard with no amount entered', () => {
    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-dashboard')).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: /stablecoin to local fiat/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'You receive' }),
    ).toBeInTheDocument();
  });

  it('shows the quote empty state until an amount is entered', () => {
    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-quote-empty')).toBeInTheDocument();
    expect(
      screen.getByText(/Enter an amount to preview Naira/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId('offramp-quote-summary'),
    ).not.toBeInTheDocument();
  });

  it('leaves the amount field empty on first paint', () => {
    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-amount')).toHaveValue('');
  });

  it('starts on the direct Stellar USDC path', () => {
    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-mode-direct')).toHaveAttribute(
      'aria-checked',
      'true',
    );
    expect(screen.getByTestId('offramp-asset-stellar-usdc')).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });

  it('renders the route rail describing how the payout moves', () => {
    render(<OfframpPage />);

    const rail = screen.getByTestId('offramp-route-rail');
    expect(rail).toBeInTheDocument();
    expect(rail).toHaveTextContent(/USDC on Stellar/i);
    expect(rail).toHaveTextContent(/Naira/i);
  });

  it('does not show the route-ready banner before a valid destination exists', () => {
    render(<OfframpPage />);

    expect(
      screen.queryByTestId('offramp-ready-banner'),
    ).not.toBeInTheDocument();
  });
});

describe('OfframpPage empty destination', () => {
  it('renders the destination form with no bank, account number or name', () => {
    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-destination-form')).toBeInTheDocument();
    expect(screen.getByText('Select your bank')).toBeInTheDocument();
    expect(screen.getByTestId('offramp-account-number')).toHaveValue('');
    expect(screen.getByTestId('offramp-account-name')).toHaveValue('');
  });

  it('reports no destination error before the user touches the account number', () => {
    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-account-number')).toHaveAttribute(
      'aria-invalid',
      'false',
    );
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('disables the payout CTA while the destination is empty', () => {
    render(<OfframpPage />);

    expect(screen.getByTestId('offramp-continue')).toBeDisabled();
  });

  it('keeps the payout CTA disabled for an amount with an empty destination', async () => {
    const user = userEvent.setup();
    render(<OfframpPage />);

    await user.type(screen.getByTestId('offramp-amount'), '100');

    // A quote exists, but there is still no bank account to pay out to.
    expect(screen.getByTestId('offramp-quote-summary')).toBeInTheDocument();
    expect(screen.getByTestId('offramp-continue')).toBeDisabled();
  });

  it('flags an invalid account number once the field is touched', async () => {
    const user = userEvent.setup();
    render(<OfframpPage />);

    await user.type(screen.getByTestId('offramp-account-number'), '12345');

    expect(screen.getByTestId('offramp-account-number')).toHaveValue('12345');
    expect(screen.getByTestId('offramp-account-number')).toHaveAttribute(
      'aria-invalid',
      'true',
    );
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Enter a valid 10-digit NUBAN account number.',
    );
    expect(screen.getByTestId('offramp-continue')).toBeDisabled();
  });

  it('masks the account number to 10 NUBAN digits', async () => {
    const user = userEvent.setup();
    render(<OfframpPage />);

    await user.type(screen.getByTestId('offramp-account-number'), '0123456789abc');

    expect(screen.getByTestId('offramp-account-number')).toHaveValue('0123456789');
  });

  it('keeps the payout CTA disabled when only the account name is filled', async () => {
    const user = userEvent.setup();
    render(<OfframpPage />);

    await user.type(screen.getByTestId('offramp-amount'), '100');
    await user.type(screen.getByTestId('offramp-account-number'), '0123456789');
    await user.type(screen.getByTestId('offramp-account-name'), 'Adebayo Okafor');

    // Valid account details, but no bank selected — CTA must stay disabled.
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByTestId('offramp-continue')).toBeDisabled();
  });
});

describe('OfframpPage institution code validation', () => {
  it('rejects 3-digit CBN bank codes as institution codes', () => {
    expect(isNigerianCbnBankCode(CBN_BANK_CODE_FIXTURE)).toBe(true);
    expect(isValidNigerianInstitutionCode(CBN_BANK_CODE_FIXTURE)).toBe(false);
  });

  it('rejects every bank code offered by the bank picker', () => {
    expect(NIGERIAN_BANKS.length).toBeGreaterThan(0);
    for (const bank of NIGERIAN_BANKS) {
      expect(isNigerianCbnBankCode(bank.code)).toBe(true);
      expect(isValidNigerianInstitutionCode(bank.code)).toBe(false);
    }
  });

  it('accepts an 8-character institution code fixture', () => {
    expect(
      isValidNigerianInstitutionCode(INSTITUTION_CODE_FIXTURE),
    ).toBe(true);
    expect(INSTITUTION_CODE_FIXTURE).toHaveLength(
      NIGERIAN_INSTITUTION_CODE_LENGTH,
    );
  });

  it('rejects institution codes that are not 8 alphanumeric characters', () => {
    expect(isValidNigerianInstitutionCode('0580000')).toBe(false);
    expect(isValidNigerianInstitutionCode('058000012')).toBe(false);
    expect(isValidNigerianInstitutionCode('0580 001')).toBe(false);
    expect(isValidNigerianInstitutionCode('0580-001')).toBe(false);
    expect(isValidNigerianInstitutionCode('')).toBe(false);
  });
});

describe('OfframpPage quote summary with fixtures', () => {
  it('renders the NGN quote summary for an indicative direct quote', async () => {
    const user = userEvent.setup();
    render(<OfframpPage />);

    await user.type(screen.getByTestId('offramp-amount'), '100');

    expect(NGN_QUOTE_FIXTURE).not.toBeNull();
    const summary = screen.getByTestId('offramp-quote-summary');
    expect(summary).toHaveTextContent(NGN_QUOTE_FIXTURE!.receiveNgn);
    expect(summary).toHaveTextContent(NGN_QUOTE_FIXTURE!.netUsdc);
    expect(summary).toHaveTextContent(NGN_QUOTE_FIXTURE!.feeUsdc);
    expect(summary).toHaveTextContent(NGN_QUOTE_FIXTURE!.sourceSymbol);
    expect(summary).toHaveTextContent(/Direct Stellar USDC/i);
    expect(
      screen.queryByTestId('offramp-quote-empty'),
    ).not.toBeInTheDocument();
  });

  it('keeps the quote labelled indicative until the payout partner is live', async () => {
    const user = userEvent.setup();
    render(<OfframpPage />);

    await user.type(screen.getByTestId('offramp-amount'), '100');

    expect(
      screen.getByText(/You receive · indicative/i),
    ).toBeInTheDocument();
  });

  it('switches the quote summary to the bridge path label', async () => {
    const user = userEvent.setup();
    render(<OfframpPage />);

    // Bridge mode plus a non-Stellar source so the summary reports the
    // bridge -> offramp path rather than the direct Stellar USDC path.
    await user.click(screen.getByTestId('offramp-mode-bridge'));
    await user.click(screen.getByTestId('offramp-asset-eth-usdc'));
    await user.type(screen.getByTestId('offramp-amount'), '100');

    expect(screen.getByTestId('offramp-asset-eth-usdc')).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(
      screen.getByText(/Bridge\s*→?\s*offramp/i),
    ).toBeInTheDocument();
  });
});

describe('OfframpPage network isolation', () => {
  it('makes no live Paycrest calls for any empty state', async () => {
    const user = userEvent.setup();
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    render(<OfframpPage />);

    await user.type(screen.getByTestId('offramp-account-number'), '0123456789');
    await user.type(screen.getByTestId('offramp-account-name'), 'Adebayo Okafor');
    await user.click(screen.getByTestId('offramp-continue'));

    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('makes no live Paycrest calls while previewing a quote', async () => {
    const user = userEvent.setup();
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    render(<OfframpPage />);

    await user.type(screen.getByTestId('offramp-amount'), '100');
    await user.click(screen.getByTestId('offramp-continue'));

    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

/** Guards the fixture itself so a broken helper fails loudly. */
describe('NGN quote fixture', () => {
  it('is a direct indicative Stellar USDC -> NGN quote', () => {
    expect(NGN_QUOTE_FIXTURE).not.toBeNull();
    expect(NGN_QUOTE_FIXTURE!.mode).toBe('direct');
    expect(NGN_QUOTE_FIXTURE!.indicative).toBe(true);
    expect(NGN_QUOTE_FIXTURE!.sourceSymbol).toBe('USDC');
    expect(NGN_QUOTE_FIXTURE!.receiveNgn).toBe('157,210.00');
  });
});

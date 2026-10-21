import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { OFFRAMP_FIAT } from '@/lib/offramp/assets';
import { NIGERIAN_BANKS } from '@/lib/offramp/nigerian-banks';
import {
  isNigerianCbnBankCode,
  isValidNigerianInstitutionCode,
  NIGERIAN_INSTITUTION_CODE_LENGTH,
  normalizeNigerianInstitutionCode,
} from '@/lib/offramp/institution-codes';

import { FiatDestinationForm } from './FiatDestinationForm';

/** Paycrest-style 8-character institution code fixture (not a 3-digit CBN code). */
const INSTITUTION_CODE_FIXTURE = '05800001';
/** Classic 3-digit CBN bank code that the Paycrest rail does NOT accept. */
const CBN_BANK_CODE_FIXTURE = '058';

type FormProps = React.ComponentProps<typeof FiatDestinationForm>;

function setup(overrides: Partial<FormProps> = {}) {
  const props: FormProps = {
    bankCode: '',
    accountNumber: '',
    accountName: '',
    onBankCodeChange: vi.fn(),
    onAccountNumberChange: vi.fn(),
    onAccountNameChange: vi.fn(),
    ...overrides,
  };
  render(<FiatDestinationForm {...props} />);
  return props;
}

/**
 * The form is fully controlled, so typing needs real state behind it to
 * exercise the input masks end to end.
 */
function ControlledForm({
  onAccountNumberChange,
  onAccountNameChange,
  overrides = {},
}: {
  onAccountNumberChange?: (value: string) => void;
  onAccountNameChange?: (value: string) => void;
  overrides?: Partial<FormProps>;
}) {
  const [bankCode, setBankCode] = useState(overrides.bankCode ?? '');
  const [accountNumber, setAccountNumber] = useState(
    overrides.accountNumber ?? '',
  );
  const [accountName, setAccountName] = useState(overrides.accountName ?? '');

  return (
    <FiatDestinationForm
      bankCode={bankCode}
      accountNumber={accountNumber}
      accountName={accountName}
      onBankCodeChange={setBankCode}
      onAccountNumberChange={(value) => {
        setAccountNumber(value);
        onAccountNumberChange?.(value);
      }}
      onAccountNameChange={(value) => {
        setAccountName(value);
        onAccountNameChange?.(value);
      }}
      accountNumberError={overrides.accountNumberError}
    />
  );
}

describe('FiatDestinationForm', () => {
  it('renders the NGN destination heading and corridor badge', () => {
    setup();

    expect(screen.getByTestId('offramp-destination-form')).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'You receive' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        `First corridor: ${OFFRAMP_FIAT.flag} ${OFFRAMP_FIAT.name} to a Nigerian bank account.`,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(`${OFFRAMP_FIAT.symbol} ${OFFRAMP_FIAT.code}`),
    ).toBeInTheDocument();
    expect(screen.getByText(OFFRAMP_FIAT.country)).toBeInTheDocument();
    expect(screen.getByText('Live corridor')).toBeInTheDocument();
  });

  it('renders the NGN destination fields with their labels', () => {
    setup();

    expect(screen.getByLabelText('Bank')).toBeInTheDocument();
    expect(screen.getByLabelText('Account number')).toBeInTheDocument();
    expect(screen.getByLabelText('Account name')).toBeInTheDocument();
    expect(screen.getByTestId('offramp-account-number')).toHaveAttribute(
      'placeholder',
      '10-digit NUBAN',
    );
    expect(screen.getByTestId('offramp-account-name')).toHaveAttribute(
      'placeholder',
      'Name on the bank account',
    );
  });

  it('offers only 3-digit CBN bank codes in the bank picker', () => {
    setup();

    // Every selectable option is a legacy 3-digit CBN code.
    for (const bank of NIGERIAN_BANKS) {
      expect(isNigerianCbnBankCode(bank.code)).toBe(true);
      expect(bank.code).toHaveLength(3);
    }

    expect(NIGERIAN_BANKS.length).toBeGreaterThan(0);
  });

  it('shows the bank placeholder until a bank is selected', () => {
    setup({ bankCode: '' });

    expect(screen.getByTestId('offramp-bank-select')).toBeInTheDocument();
    expect(screen.getByText('Select your bank')).toBeInTheDocument();
  });

  it('shows the selected bank name once a bank code is set', () => {
    setup({ bankCode: '044' });

    expect(screen.getByText('Access Bank')).toBeInTheDocument();
    expect(screen.queryByText('Select your bank')).not.toBeInTheDocument();
  });

  it('sanitizes the account number to 10 NUBAN digits', async () => {
    const user = userEvent.setup();
    const onAccountNumberChange = vi.fn();
    render(
      <ControlledForm
        onAccountNumberChange={onAccountNumberChange}
        overrides={{ bankCode: '044' }}
      />,
    );

    await user.type(screen.getByTestId('offramp-account-number'), '12a3b4567890');

    // Non-digits are stripped and the value is capped at 10 NUBAN digits.
    expect(screen.getByTestId('offramp-account-number')).toHaveValue('1234567890');
    expect(onAccountNumberChange).toHaveBeenLastCalledWith('1234567890');
  });

  it('reports no account number error while the field is valid', () => {
    setup({ accountNumber: '0123456789' });

    const input = screen.getByTestId('offramp-account-number');
    expect(input).toHaveAttribute('aria-invalid', 'false');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(
      screen.getByText('Nigerian bank accounts use a 10-digit NUBAN.'),
    ).toBeInTheDocument();
  });

  it('surfaces the account number error with aria-invalid', () => {
    setup({
      accountNumber: '12345',
      accountNumberError: 'Enter a valid 10-digit NUBAN account number.',
    });

    const input = screen.getByTestId('offramp-account-number');
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Enter a valid 10-digit NUBAN account number.',
    );
    expect(
      screen.queryByText('Nigerian bank accounts use a 10-digit NUBAN.'),
    ).not.toBeInTheDocument();
  });

  it('emits account name changes verbatim', async () => {
    const user = userEvent.setup();
    const onAccountNameChange = vi.fn();
    render(
      <ControlledForm
        onAccountNameChange={onAccountNameChange}
        overrides={{ bankCode: '044' }}
      />,
    );

    await user.type(screen.getByTestId('offramp-account-name'), 'Adebayo Okafor');

    // Unlike the account number, the account name is not sanitized.
    expect(screen.getByTestId('offramp-account-name')).toHaveValue(
      'Adebayo Okafor',
    );
    expect(onAccountNameChange).toHaveBeenLastCalledWith('Adebayo Okafor');
  });

  it('does not make any Paycrest or bank network calls', () => {
    const spy = vi.spyOn(globalThis, 'fetch');

    setup({
      bankCode: '044',
      accountNumber: '0123456789',
      accountName: 'Adebayo Okafor',
    });

    expect(spy).not.toHaveBeenCalled();
  });
});

describe('institution code contract for the NGN offramp destination', () => {
  it('rejects 3-digit CBN bank codes as institution codes', () => {
    expect(isValidNigerianInstitutionCode(CBN_BANK_CODE_FIXTURE)).toBe(false);
    expect(NIGERIAN_BANKS.every(
      (bank) => !isValidNigerianInstitutionCode(bank.code),
    )).toBe(true);
  });

  it('accepts an 8-character institution code fixture', () => {
    expect(isValidNigerianInstitutionCode(INSTITUTION_CODE_FIXTURE)).toBe(true);
    expect(INSTITUTION_CODE_FIXTURE).toHaveLength(
      NIGERIAN_INSTITUTION_CODE_LENGTH,
    );
  });

  it('rejects institution codes that are not 8 alphanumeric characters', () => {
    expect(isValidNigerianInstitutionCode('')).toBe(false);
    expect(isValidNigerianInstitutionCode('0580000')).toBe(false); // 7 chars
    expect(isValidNigerianInstitutionCode('058000012')).toBe(false); // 9 chars
    expect(isValidNigerianInstitutionCode('0580 001')).toBe(false); // space
    expect(isValidNigerianInstitutionCode('0580-001')).toBe(false); // dash
    expect(isValidNigerianInstitutionCode('0580000!')).toBe(false); // symbol
  });

  it('normalizes typed input to uppercase and caps the length', () => {
    expect(normalizeNigerianInstitutionCode(' 0580ab01 ')).toBe('0580AB01');
    expect(normalizeNigerianInstitutionCode('058000012345')).toBe('05800001');
  });
});

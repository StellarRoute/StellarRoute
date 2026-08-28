import { useState } from 'react';
import type { Story } from '@ladle/react';
import '@/app/globals.css';
import { FiatDestinationForm } from './FiatDestinationForm';

/** Empty state — no bank selected, all fields blank. */
export const Empty: Story = () => {
  const [bankCode, setBankCode] = useState('');
  const [accountNumber, setAccountNumber] = useState('');
  const [accountName, setAccountName] = useState('');

  return (
    <div className="dark min-h-screen bg-background text-foreground p-8">
      <div className="mx-auto max-w-md">
        <FiatDestinationForm
          bankCode={bankCode}
          accountNumber={accountNumber}
          accountName={accountName}
          onBankCodeChange={setBankCode}
          onAccountNumberChange={setAccountNumber}
          onAccountNameChange={setAccountName}
        />
      </div>
    </div>
  );
};
Empty.storyName = 'Destination Form — Empty';

/** Filled state — all fields populated with fixture data. */
export const Filled: Story = () => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="mx-auto max-w-md">
      <FiatDestinationForm
        bankCode="044"
        accountNumber="1234567890"
        accountName="Adebayo Okafor"
        onBankCodeChange={() => {}}
        onAccountNumberChange={() => {}}
        onAccountNameChange={() => {}}
      />
    </div>
  </div>
);
Filled.storyName = 'Destination Form — Filled';

/** Validation error — invalid account number format. */
export const ValidationError: Story = () => {
  const [bankCode, setBankCode] = useState('044');
  const [accountNumber, setAccountNumber] = useState('12345');
  const [accountName, setAccountName] = useState('Adebayo Okafor');

  return (
    <div className="dark min-h-screen bg-background text-foreground p-8">
      <div className="mx-auto max-w-md">
        <FiatDestinationForm
          bankCode={bankCode}
          accountNumber={accountNumber}
          accountName={accountName}
          onBankCodeChange={setBankCode}
          onAccountNumberChange={setAccountNumber}
          onAccountNameChange={setAccountName}
          accountNumberError="Enter a valid 10-digit NUBAN account number."
        />
      </div>
    </div>
  );
};
ValidationError.storyName = 'Destination Form — Validation Error';

import React from 'react';
import { CopyButton } from './CopyButton';

const meta = {
  title: 'Shared/CopyButton',
};

export default meta;

export const Default = () => (
  <div className="flex items-center gap-2 p-4">
    <span className="text-sm font-mono">GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN</span>
    <CopyButton
      value="GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
      label="Copy public key"
    />
  </div>
);

export const TransactionHash = () => (
  <div className="flex items-center gap-2 p-4">
    <span className="text-sm font-mono">0x4a9b...7c2d</span>
    <CopyButton
      value="0x4a9b1c3d5e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b"
      label="Copy transaction hash"
    />
  </div>
);

export const CustomLabel = () => (
  <div className="flex items-center gap-2 p-4">
    <span className="text-sm">Deposit Memo: 123456789</span>
    <CopyButton
      value="123456789"
      label="Copy memo"
    />
  </div>
);

export const InsideCard = () => (
  <div className="max-w-sm rounded-lg border p-4 shadow-sm space-y-2">
    <p className="text-xs text-muted-foreground">Recipient Address</p>
    <div className="flex items-center justify-between rounded bg-muted p-2">
      <span className="text-xs font-mono truncate mr-2">
        GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
      </span>
      <CopyButton
        value="GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
        label="Copy recipient address"
      />
    </div>
  </div>
);

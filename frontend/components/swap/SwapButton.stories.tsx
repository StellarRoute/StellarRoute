import type { Story } from '@ladle/react';
import '@/app/globals.css';
import { SwapButton, type SwapButtonState } from './SwapButton';

function Frame({ children }: { children: React.ReactNode }) {
  return (
    <div className="dark min-h-screen bg-background text-foreground p-8">
      <div className="max-w-sm mx-auto">{children}</div>
    </div>
  );
}

const states: Array<[SwapButtonState, string]> = [
  ['no_wallet', 'No Wallet'],
  ['no_amount', 'No Amount'],
  ['insufficient_balance', 'Insufficient Balance'],
  ['high_price_impact', 'High Price Impact'],
  ['high_impact_warning', 'High Impact Warning'],
  ['slippage_ack_required', 'Slippage Ack Required'],
  ['refreshing_quote', 'Refreshing Quote'],
  ['ready', 'Ready'],
  ['executing', 'Executing'],
  ['error', 'Error'],
  ['permission_blocked', 'Permission Blocked'],
];

export const NoWallet: Story = () => (
  <Frame>
    <SwapButton state="no_wallet" onSwap={() => {}} onConnectWallet={() => {}} />
  </Frame>
);

export const NoAmount: Story = () => (
  <Frame>
    <SwapButton state="no_amount" onSwap={() => {}} />
  </Frame>
);

export const InsufficientBalance: Story = () => (
  <Frame>
    <SwapButton state="insufficient_balance" onSwap={() => {}} />
  </Frame>
);

export const HighPriceImpact: Story = () => (
  <Frame>
    <SwapButton state="high_price_impact" onSwap={() => {}} />
  </Frame>
);

export const HighImpactWarning: Story = () => (
  <Frame>
    <SwapButton state="high_impact_warning" onSwap={() => {}} />
  </Frame>
);

export const SlippageAckRequired: Story = () => (
  <Frame>
    <SwapButton state="slippage_ack_required" onSwap={() => {}} />
  </Frame>
);

export const RefreshingQuote: Story = () => (
  <Frame>
    <SwapButton state="refreshing_quote" onSwap={() => {}} />
  </Frame>
);

export const Ready: Story = () => (
  <Frame>
    <SwapButton state="ready" onSwap={() => {}} />
  </Frame>
);

export const Executing: Story = () => (
  <Frame>
    <SwapButton state="executing" onSwap={() => {}} />
  </Frame>
);

export const ErrorState: Story = () => (
  <Frame>
    <SwapButton state="error" onSwap={() => {}} />
  </Frame>
);

export const PermissionBlocked: Story = () => (
  <Frame>
    <SwapButton state="permission_blocked" onSwap={() => {}} />
  </Frame>
);

/** Full state matrix at a glance, with disabled states showing their tooltip reason on hover. */
export const StateMatrix: Story = () => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="grid gap-6 max-w-sm mx-auto">
      {states.map(([state, label]) => (
        <section key={state} className="space-y-2">
          <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
            {label}
          </h3>
          <SwapButton
            state={state}
            onSwap={() => {}}
            onConnectWallet={() => {}}
          />
        </section>
      ))}
    </div>
  </div>
);

import type { Story } from '@ladle/react';
import '@/app/globals.css';
import { OfframpQuoteSummary } from './OfframpQuoteSummary';
import type { OfframpQuotePreview } from '@/lib/offramp/types';

const fixtureQuote: OfframpQuotePreview = {
  sourceAmount: '100.00',
  sourceSymbol: 'USDC',
  usdcAmount: '100.000000',
  feeUsdc: '0.500000',
  netUsdc: '99.500000',
  rateNgn: 1_580,
  receiveNgn: '157,210.00',
  etaLabel: 'Usually under 15 minutes once payout is live',
  mode: 'direct',
  indicative: true,
};

const bridgeQuote: OfframpQuotePreview = {
  sourceAmount: '50.00',
  sourceSymbol: 'USDC',
  usdcAmount: '50.000000',
  feeUsdc: '0.250000',
  netUsdc: '49.750000',
  rateNgn: 1_580,
  receiveNgn: '78,605.00',
  etaLabel: 'Bridge finality + payout — typically 20–45 minutes',
  mode: 'bridge',
  indicative: true,
};

/** Empty state — no quote yet, prompt to enter an amount. */
export const Empty: Story = () => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="mx-auto max-w-sm">
      <OfframpQuoteSummary quote={null} />
    </div>
  </div>
);
Empty.storyName = 'Quote Summary — Empty';

/** Filled state — direct Stellar USDC path with indicative quote. */
export const DirectQuote: Story = () => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="mx-auto max-w-sm">
      <OfframpQuoteSummary quote={fixtureQuote} />
    </div>
  </div>
);
DirectQuote.storyName = 'Quote Summary — Direct USDC';

/** Filled state — bridge path with indicative quote. */
export const BridgeQuote: Story = () => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="mx-auto max-w-sm">
      <OfframpQuoteSummary quote={bridgeQuote} />
    </div>
  </div>
);
BridgeQuote.storyName = 'Quote Summary — Bridge Path';

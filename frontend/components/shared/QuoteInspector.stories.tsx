import type { Story } from '@ladle/react';
import type { ReactNode } from 'react';
import '@/app/globals.css';
import { QuoteInspector, type VenueQuote } from './QuoteInspector';
import type { Asset } from '@/types';

const XLM: Asset = { asset_type: 'native' };
const USDC: Asset = {
  asset_type: 'credit_alphanum4',
  asset_code: 'USDC',
  asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
};

const TIMESTAMP = Date.UTC(2025, 0, 1, 12, 0, 0);

/** Classic one-hop SDEX quote — the live, supported path. */
const classicSdexQuote: VenueQuote = {
  venueName: 'SDEX',
  base_asset: XLM,
  quote_asset: USDC,
  amount: '1000',
  price: '0.112500',
  total: '112.500000',
  quote_type: 'sell',
  midpoint: '0.112600',
  spread_bps: 9,
  path: [{ from_asset: XLM, to_asset: USDC, price: '0.112500', source: 'sdex' }],
  priceImpact: '0.04',
  timestamp: TIMESTAMP,
  reliabilityScore: 0.98,
};

/** Display-only AMM fixture — an unsupported/excluded venue. */
const unsupportedAmmQuote: VenueQuote = {
  ...classicSdexQuote,
  venueName: 'AMM Pool (unsupported)',
  price: '0.111800',
  total: '111.800000',
  degraded: true,
  isAggregated: true,
  reliabilityScore: 0.42,
  path: [
    {
      from_asset: XLM,
      to_asset: USDC,
      price: '0.111800',
      source: 'amm:CA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ',
      fee_bps: 30,
    },
  ],
};

const Frame = ({ children }: { children: ReactNode }) => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="mx-auto max-w-4xl">{children}</div>
  </div>
);

/** Loading skeleton for the operator inspector. */
export const Loading: Story = () => (
  <Frame>
    <QuoteInspector quotes={[]} onSelect={() => {}} isLoading />
  </Frame>
);
Loading.storyName = 'Quote Inspector — Loading';

/** Single classic one-hop SDEX quote. */
export const ClassicSdex: Story = () => (
  <Frame>
    <QuoteInspector quotes={[classicSdexQuote]} onSelect={() => {}} />
  </Frame>
);
ClassicSdex.storyName = 'Quote Inspector — Classic SDEX';

/** Classic SDEX alongside a display-only unsupported AMM venue. */
export const UnsupportedAmmVenue: Story = () => (
  <Frame>
    <QuoteInspector
      quotes={[classicSdexQuote, unsupportedAmmQuote]}
      onSelect={() => {}}
    />
  </Frame>
);
UnsupportedAmmVenue.storyName = 'Quote Inspector — Unsupported AMM Venue';

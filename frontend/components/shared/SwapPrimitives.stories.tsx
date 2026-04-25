import { PathStep } from '@/types';
import { TokenSelector, QuoteCard, RouteRow, SlippageControl } from './index';
import { useState } from 'react';

const sampleOptions = [
  { value: 'XLM', label: 'Stellar Lumens', symbol: 'XLM' },
  { value: 'USDC', label: 'USD Coin', symbol: 'USDC' },
  { value: 'BTC', label: 'Bitcoin', symbol: 'BTC' },
];

const sampleRouteStep: PathStep = {
  from_asset: { asset_type: 'native' },
  to_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GA5Z...' },
  price: '0.105',
  source: 'sdex',
};

const samplePath: PathStep[] = [sampleRouteStep];

const meta = {
  title: 'Swap primitives',
};

export default meta;

export const TokenSelectorDefault = () => {
  const [selectedToken, setSelectedToken] = useState('XLM');

  return (
    <TokenSelector
      value={selectedToken}
      options={sampleOptions}
      onChange={setSelectedToken}
    />
  );
};

export const TokenSelectorLoading = () => <TokenSelector options={[]} loading onChange={() => {}} />;
export const TokenSelectorError = () => <TokenSelector options={[]} error="Network failed" onChange={() => {}} />;
export const TokenSelectorEmpty = () => <TokenSelector options={[]} onChange={() => {}} />;

export const QuoteCardDefault = () => (
  <QuoteCard
    fromAmount="100"
    toAmount="10.5"
    price="0.105"
    slippage={0.5}
    path={samplePath}
  />
);

export const QuoteCardLoading = () => <QuoteCard isLoading />;
export const QuoteCardError = () => <QuoteCard error="Quote request failed" />;
export const QuoteCardEmpty = () => <QuoteCard fromAmount="" toAmount="" price="" />;

export const RouteRowDefault = () => <RouteRow step={sampleRouteStep} />;
export const RouteRowLoading = () => <RouteRow isLoading />;
export const RouteRowError = () => <RouteRow error="Route fetch failed" />;
export const RouteRowEmpty = () => <RouteRow />;

export const SlippageControlDefault = () => {
  const [value, setValue] = useState(0.5);

  return <SlippageControl value={value} onChange={setValue} />;
};

export const SlippageControlLoading = () => <SlippageControl value={0} onChange={() => {}} isLoading />;
export const SlippageControlError = () => <SlippageControl value={5} onChange={() => {}} error="Invalid slippage" />;
export const SlippageControlEmpty = () => <SlippageControl value={0} onChange={() => {}} />;

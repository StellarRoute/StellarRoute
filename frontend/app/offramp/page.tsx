import { OfframpPageClient } from './OfframpPageClient';
import { buildPageMetadata } from '@/lib/seo';

export const metadata = buildPageMetadata({
  title: 'Offramp to Naira',
  description:
    'Cash out Stellar USDC — or bridge then offramp stablecoins — to Nigerian Naira (NGN). Non-custodial Stellar path into local fiat.',
  path: '/offramp',
});

export default function OfframpPage() {
  return (
    <div className="mx-auto w-full max-w-5xl py-2 sm:py-4">
      <OfframpPageClient />
    </div>
  );
}

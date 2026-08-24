import { buildPageMetadata } from '@/lib/seo';

export const metadata = buildPageMetadata({
  title: 'Stellar DEX Orderbook',
  description:
    'Live order book and market depth for Stellar DEX trading pairs on StellarRoute.',
  path: '/orderbook',
});

export default function OrderbookLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}

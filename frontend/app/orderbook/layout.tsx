import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'Orderbook | StellarRoute',
  description: 'Live order book and market depth for Stellar trading pairs.',
  openGraph: {
    title: 'Orderbook | StellarRoute',
    description:
      'Real-time market depth and order book data for Stellar DEX pairs.',
    type: 'website',
  },
};

export default function OrderbookLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}

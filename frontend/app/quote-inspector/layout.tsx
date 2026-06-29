import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'Quote Inspector | StellarRoute',
  description:
    'Reconcile and compare quotes from multiple Stellar liquidity venues.',
  openGraph: {
    title: 'Quote Inspector | StellarRoute',
    description:
      'Analyze and reconcile quotes across SDEX and Soroban AMM pools.',
    type: 'website',
  },
};

export default function QuoteInspectorLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}

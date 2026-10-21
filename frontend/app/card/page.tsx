import { notFound } from 'next/navigation';

import { buildPageMetadata } from '@/lib/seo';

import { CardPageClient } from './CardPageClient';

function isCardEnabled(): boolean {
  const value = process.env.NEXT_PUBLIC_CARD_ENABLED;
  if (!value) return false;
  const normalized = value.trim().toLowerCase();
  return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on';
}

export const metadata = buildPageMetadata({
  title: 'Card Controls',
  description: 'Freeze or unfreeze an additive card program. This feature is disabled by default.',
  path: '/card',
  absoluteTitle: true,
});

export default function CardPage() {
  if (!isCardEnabled()) {
    notFound();
  }

  return <CardPageClient />;
}

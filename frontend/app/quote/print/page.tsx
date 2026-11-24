import { Suspense } from 'react';
import type { Metadata } from 'next';

import { QuotePrintPageClient } from './QuotePrintPageClient';

export const metadata: Metadata = {
  title: 'Print Quote | StellarRoute',
  description: 'Print-friendly summary of a StellarRoute swap quote.',
  robots: { index: false, follow: false },
};

export default function QuotePrintPage() {
  return (
    <Suspense fallback={null}>
      <QuotePrintPageClient />
    </Suspense>
  );
}

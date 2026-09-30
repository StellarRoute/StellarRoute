'use client';

import { useMemo } from 'react';
import { useSearchParams } from 'next/navigation';

import { PrintHeader } from './PrintHeader';
import {
  parseQuotePrintPayload,
  QUOTE_PRINT_QUERY_PARAM,
} from './quote-print-payload';

/**
 * Scoped to this route only: this <style> tag is only ever emitted into the
 * page when /quote/print renders, so it cannot affect print output (or
 * anything else) on /swap or any other route. It hides the shared site
 * header/footer (rendered by the root AppShell, which this PR does not
 * modify) while printing, and hides the on-screen "Print" button from the
 * printed output.
 */
function PrintScopedStyles() {
  return (
    <style>{`
      @media print {
        header, footer { display: none !important; }
        [data-testid="quote-print-screen-only"] { display: none !important; }
        @page { size: portrait; margin: 1.5cm; }
      }
    `}</style>
  );
}

export function QuotePrintPageClient() {
  const searchParams = useSearchParams();
  const encoded = searchParams.get(QUOTE_PRINT_QUERY_PARAM);

  const payload = useMemo(() => parseQuotePrintPayload(encoded), [encoded]);

  if (!payload) {
    return (
      <div className="mx-auto max-w-2xl py-10 text-center">
        <PrintScopedStyles />
        <h1 className="text-lg font-semibold">No quote to print</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          This page renders a quote that was already fetched elsewhere in the
          app. Open it via a print link that includes a valid{' '}
          <code className="rounded bg-muted px-1 py-0.5">
            ?{QUOTE_PRINT_QUERY_PARAM}=
          </code>{' '}
          value.
        </p>
      </div>
    );
  }

  const { exportedAt, market, pricing, route } = payload;

  return (
    <div
      data-testid="quote-print-page"
      className="mx-auto w-full max-w-2xl py-6 print:max-w-full print:py-0"
    >
      <PrintScopedStyles />

      <div
        data-testid="quote-print-screen-only"
        className="mb-4 flex justify-end"
      >
        <button
          type="button"
          onClick={() => window.print()}
          className="rounded-md border border-border px-3 py-1.5 text-sm font-medium hover:bg-muted"
        >
          Print
        </button>
      </div>

      <PrintHeader
        capturedAt={exportedAt}
        fromSymbol={market.fromAsset}
        toSymbol={market.toAsset}
      />

      <section className="mb-6 print:break-inside-avoid">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-black/60">
          Market
        </h2>
        <dl className="space-y-1 text-sm">
          <Row label="You pay" value={`${market.fromAmount} ${market.fromAsset}`} />
          <Row
            label="You receive (est.)"
            value={`${market.expectedToAmount} ${market.toAsset}`}
          />
        </dl>
      </section>

      <section className="mb-6 print:break-inside-avoid">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-black/60">
          Pricing
        </h2>
        <dl className="space-y-1 text-sm">
          <Row label="Rate" value={pricing.rate} />
          <Row label="Price impact" value={pricing.priceImpactPct} />
          <Row label="Minimum received" value={pricing.minimumReceived} />
          <Row label="Network fee" value={pricing.networkFee} />
        </dl>
      </section>

      <section className="print:break-inside-avoid">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-black/60">
          Route
        </h2>
        <dl className="space-y-1 text-sm">
          <Row label="Venue" value={route.selectedVenue} />
          <Row label="Path" value={route.routeSummary} />
        </dl>
      </section>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4">
      <dt className="text-black/60">{label}</dt>
      <dd className="text-right font-medium">{value}</dd>
    </div>
  );
}

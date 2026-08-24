'use client';

export function RouteDisclosurePanel() {
  return (
    <details
      className="group rounded-2xl border border-border/40 bg-muted/15 text-sm open:pb-4"
      data-testid="route-disclosure-panel"
    >
      <summary
        className="flex min-h-11 cursor-pointer list-none items-center gap-2 px-4 py-3 font-semibold text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring [&::-webkit-details-marker]:hidden"
      >
        <span>Before you route</span>
        <span className="ml-auto text-xs font-normal text-muted-foreground group-open:hidden">
          Show details
        </span>
        <span className="ml-auto hidden text-xs font-normal text-muted-foreground group-open:inline">
          Hide details
        </span>
      </summary>
      <ul
        className="space-y-2 px-4 pt-1 text-muted-foreground list-disc pl-9"
        aria-label="Cross-chain risk disclosures"
      >
        <li>
          StellarRoute is non-custodial — you sign with your own wallets; we never
          hold keys.
        </li>
        <li>
          Cross-chain moves burn on the source chain before minting on the
          destination. Funds are not spendable on both sides during attestation.
        </li>
        <li>
          Attestation and finality times vary by corridor and network conditions.
        </li>
        <li>
          Only corridors marked executable can proceed to review and signing.
          Preview corridors show protocol steps without live quotes.
        </li>
      </ul>
    </details>
  );
}

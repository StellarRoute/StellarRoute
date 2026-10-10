interface PrintHeaderProps {
  /** ISO timestamp string captured when the quote payload was built. */
  capturedAt: string;
  fromSymbol: string;
  toSymbol: string;
}

/** Formats an ISO timestamp as `YYYY-MM-DD HH:mm UTC`. Pure + unit-testable. */
export function formatPrintTimestamp(isoString: string): string {
  const date = new Date(isoString);
  if (Number.isNaN(date.getTime())) return isoString;

  const pad = (value: number) => value.toString().padStart(2, '0');
  const year = date.getUTCFullYear();
  const month = pad(date.getUTCMonth() + 1);
  const day = pad(date.getUTCDate());
  const hours = pad(date.getUTCHours());
  const minutes = pad(date.getUTCMinutes());

  return `${year}-${month}-${day} ${hours}:${minutes} UTC`;
}

/** Simple heading row for the printed page: brand, pair, and capture time. */
export function PrintHeader({
  capturedAt,
  fromSymbol,
  toSymbol,
}: PrintHeaderProps) {
  return (
    <div className="mb-6 flex items-baseline justify-between border-b border-black/20 pb-3">
      <span className="text-lg font-semibold">StellarRoute</span>
      <div className="text-right text-sm">
        <div className="font-medium">
          {fromSymbol} &rarr; {toSymbol}
        </div>
        <div className="text-black/60">
          {formatPrintTimestamp(capturedAt)}
        </div>
      </div>
    </div>
  );
}

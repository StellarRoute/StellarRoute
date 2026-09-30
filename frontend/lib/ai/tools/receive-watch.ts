export const RECEIVE_WATCH_INTERVAL_MS = 5_000;

const DEFAULT_HORIZON_URL = "https://horizon.stellar.org";
const PAGE_LIMIT = 200;

type HorizonPayment = {
  id?: string;
  paging_token?: string;
  type?: string;
  to?: string;
  amount?: string;
  asset_type?: string;
  asset_code?: string;
};

type PaymentsResponse = {
  _embedded?: {
    records?: HorizonPayment[];
  };
};

export type ReceiveWatchOptions = {
  horizonUrl?: string;
  intervalMs?: number;
  fetchImpl?: typeof fetch;
};

function formatIncomingPayment(payment: HorizonPayment): string | null {
  if (
    payment.type !== "payment" &&
    payment.type !== "path_payment" &&
    payment.type !== "path_payment_strict_send" &&
    payment.type !== "path_payment_strict_receive"
  ) {
    return null;
  }

  if (!payment.amount || !payment.to) return null;

  const asset = payment.asset_type === "native" ? "XLM" : payment.asset_code;
  if (!asset) return null;

  return `Received ${payment.amount} ${asset}`;
}

/** Polls public Horizon payments and reports incoming payments after subscription. */
export function watchIncomingPayments(
  accountId: string,
  appendLine: (line: string) => void,
  options: ReceiveWatchOptions = {},
): () => void {
  const horizonUrl = (options.horizonUrl ?? DEFAULT_HORIZON_URL).replace(/\/$/, "");
  const intervalMs = options.intervalMs ?? RECEIVE_WATCH_INTERVAL_MS;
  const fetchImpl = options.fetchImpl ?? fetch;
  const controller = new AbortController();
  let cursor: string | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let stopped = false;
  let initialized = false;

  const poll = async () => {
    const query = new URLSearchParams({
      order: cursor ? "asc" : "desc",
      limit: String(cursor ? PAGE_LIMIT : 1),
    });
    if (cursor) query.set("cursor", cursor);

    try {
      const response = await fetchImpl(
        `${horizonUrl}/accounts/${encodeURIComponent(accountId)}/payments?${query}`,
        { signal: controller.signal },
      );
      if (stopped) return;
      if (!response.ok) throw new Error(`Horizon returned ${response.status}`);

      const payload = (await response.json()) as PaymentsResponse;
      if (stopped) return;
      const records = payload._embedded?.records ?? [];

      if (!initialized) {
        cursor = records[0]?.paging_token ?? null;
        initialized = true;
      } else {
        for (const payment of records) {
          if (stopped) return;
          if (payment.paging_token) cursor = payment.paging_token;
          if (payment.to !== accountId) continue;

          const line = formatIncomingPayment(payment);
          if (line) appendLine(line);
        }
      }
    } catch {
      if (stopped || controller.signal.aborted) return;
    }

    if (!stopped) timer = setTimeout(poll, intervalMs);
  };

  void poll();

  return () => {
    stopped = true;
    controller.abort();
    if (timer !== undefined) clearTimeout(timer);
  };
}
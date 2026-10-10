export const BRIDGE_UNAVAILABLE_MESSAGE =
  "Bridge quotes are unavailable in this environment because CCTP is not enabled.";

export type BridgeIntentResult<Quote> =
  | { available: false; message: string }
  | { available: true; quote: Quote };

export type BridgeIntentDependencies<Request, Readiness, Quote> = {
  checkReadiness: () => Promise<Readiness>;
  isEnabled: (readiness: Readiness) => boolean;
  getQuote: (request: Request) => Promise<Quote>;
};

export type BridgeConfirmation = {
  source: string;
  destination: string;
  amount: string | number;
};

export function buildBridgeConfirmationUrl(
  confirmation: BridgeConfirmation,
): string {
  const query = new URLSearchParams({
    source: confirmation.source,
    destination: confirmation.destination,
    amount: String(confirmation.amount),
  });

  return `/cross-chain-swap?${query.toString()}`;
}

/** Hands a confirmed bridge intent to the existing cross-chain page. */
export function confirmBridgeIntent(
  confirmation: BridgeConfirmation,
  navigate: (url: string) => void = (url) => window.location.assign(url),
): string {
  const url = buildBridgeConfirmationUrl(confirmation);
  navigate(url);
  return url;
}

/** Checks CCTP readiness before invoking the existing bridge quote client. */
export async function runBridgeIntent<Request, Readiness, Quote>(
  request: Request,
  dependencies: BridgeIntentDependencies<Request, Readiness, Quote>,
): Promise<BridgeIntentResult<Quote>> {
  let readiness: Readiness;

  try {
    readiness = await dependencies.checkReadiness();
  } catch {
    return { available: false, message: BRIDGE_UNAVAILABLE_MESSAGE };
  }

  if (!dependencies.isEnabled(readiness)) {
    return { available: false, message: BRIDGE_UNAVAILABLE_MESSAGE };
  }

  // Do not catch quote errors; failed quotes or attestations must not look successful.
  const quote = await dependencies.getQuote(request);
  return { available: true, quote };
}
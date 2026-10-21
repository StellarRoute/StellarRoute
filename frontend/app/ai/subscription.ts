export const AI_SUBSCRIPTIONS_STORAGE_KEY = "stellarroute:ai:subscriptions:v1";

export type AiSubscription = {
  id: string;
  source: string;
  destination: string;
  amount: string;
  nextDueAt: string;
};

export function isSubscriptionDue(
  subscription: AiSubscription,
  nowMs: number,
): boolean {
  const dueAtMs = Date.parse(subscription.nextDueAt);
  return Number.isFinite(dueAtMs) && dueAtMs <= nowMs;
}

export function findDueSubscription(
  subscriptions: AiSubscription[],
  nowMs: number,
): AiSubscription | null {
  return (
    subscriptions
      .filter((subscription) => isSubscriptionDue(subscription, nowMs))
      .sort(
        (left, right) =>
          Date.parse(left.nextDueAt) - Date.parse(right.nextDueAt),
      )[0] ?? null
  );
}

export function parseSubscriptions(value: string | null): AiSubscription[] {
  if (!value) return [];

  try {
    const parsed: unknown = JSON.parse(value);
    if (!Array.isArray(parsed)) return [];

    return parsed.filter(
      (subscription): subscription is AiSubscription =>
        typeof subscription === "object" &&
        subscription !== null &&
        typeof subscription.id === "string" &&
        typeof subscription.source === "string" &&
        typeof subscription.destination === "string" &&
        typeof subscription.amount === "string" &&
        typeof subscription.nextDueAt === "string",
    );
  } catch {
    return [];
  }
}
import { afterEach, describe, expect, it, vi } from "vitest";

import { findDueSubscription, isSubscriptionDue } from "./subscription";

const NOW = new Date("2026-09-28T12:00:00.000Z");

function subscription(id: string, nextDueAt: string) {
  return {
    id,
    source: "native",
    destination: "USDC:ISSUER",
    amount: "25",
    nextDueAt,
  };
}

describe("AI subscription due checks", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("considers a subscription due at its scheduled time", () => {
    vi.useFakeTimers();
    vi.setSystemTime(NOW);

    expect(
      isSubscriptionDue(subscription("due", NOW.toISOString()), Date.now()),
    ).toBe(true);
    expect(
      isSubscriptionDue(
        subscription("future", "2026-09-28T12:00:01.000Z"),
        Date.now(),
      ),
    ).toBe(false);
  });

  it("selects only the earliest due subscription", () => {
    vi.useFakeTimers();
    vi.setSystemTime(NOW);
    const subscriptions = [
      subscription("later-due", "2026-09-28T11:00:00.000Z"),
      subscription("earliest-due", "2026-09-27T11:00:00.000Z"),
      subscription("not-due", "2026-09-29T11:00:00.000Z"),
    ];

    expect(findDueSubscription(subscriptions, Date.now())?.id).toBe(
      "earliest-due",
    );
  });
});
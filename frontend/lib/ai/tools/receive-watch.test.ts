import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  RECEIVE_WATCH_INTERVAL_MS,
  watchIncomingPayments,
} from "./receive-watch";

const ACCOUNT_ID = "GABCDEFGH";

function payment(
  pagingToken: string,
  values: Record<string, string> = {},
) {
  return {
    id: pagingToken,
    paging_token: pagingToken,
    type: "payment",
    to: ACCOUNT_ID,
    amount: "12.5",
    asset_type: "native",
    ...values,
  };
}

function response(records: ReturnType<typeof payment>[]) {
  return {
    ok: true,
    status: 200,
    json: async () => ({ _embedded: { records } }),
  } as Response;
}

async function flushPromises() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("watchIncomingPayments", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("reports new incoming payments without replaying the initial snapshot", async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(response([payment("10")]))
      .mockResolvedValueOnce(
        response([
          payment("11"),
          payment("12", { asset_type: "credit_alphanum4", asset_code: "USD" }),
          payment("13", { to: "GOTHER" }),
          payment("14", { type: "path_payment_strict_receive" }),
        ]),
      );
    const appendLine = vi.fn();

    watchIncomingPayments(ACCOUNT_ID, appendLine, { fetchImpl });
    await flushPromises();
    expect(appendLine).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(RECEIVE_WATCH_INTERVAL_MS);

    expect(appendLine.mock.calls).toEqual([
      ["Received 12.5 XLM"],
      ["Received 12.5 USD"],
      ["Received 12.5 XLM"],
    ]);
    expect(fetchImpl.mock.calls[1][0]).toContain("cursor=10");
    expect(fetchImpl.mock.calls[1][0]).toContain("order=asc");
  });

  it("stops polling and ignores an in-flight response after unsubscribe", async () => {
    let resolveFetch: (response: Response) => void = () => {};
    const fetchImpl = vi.fn<typeof fetch>().mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveFetch = resolve;
        }),
    );
    const appendLine = vi.fn();
    const unsubscribe = watchIncomingPayments(ACCOUNT_ID, appendLine, {
      fetchImpl,
    });

    unsubscribe();
    resolveFetch(response([payment("10")]));
    await flushPromises();
    await vi.advanceTimersByTimeAsync(RECEIVE_WATCH_INTERVAL_MS * 2);

    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(appendLine).not.toHaveBeenCalled();
    expect(fetchImpl.mock.calls[0][1]?.signal?.aborted).toBe(true);
  });
});
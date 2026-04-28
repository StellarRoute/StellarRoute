import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSystemMessages, type SystemMessage } from "./useSystemMessages";

const SAMPLE: SystemMessage[] = [
  {
    id: "msg-1",
    title: "Scheduled maintenance",
    body: "The service will be down for 30 minutes.",
    severity: "maintenance",
    created_at: "2026-04-26T10:00:00Z",
  },
  {
    id: "msg-2",
    title: "New feature available",
    body: "Multi-hop routing is now live.",
    severity: "info",
    created_at: "2026-04-25T08:00:00Z",
  },
];

describe("useSystemMessages", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => SAMPLE,
      })
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches and returns messages", async () => {
    const { result } = renderHook(() => useSystemMessages());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.messages).toHaveLength(2);
    expect(result.current.unreadCount).toBe(2);
  });

  it("handles fetch failure gracefully without throwing", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new Error("network error"))
    );
    const { result } = renderHook(() => useSystemMessages());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // Error is surfaced but messages remain empty — core swap is not blocked
    expect(result.current.error).not.toBeNull();
    expect(result.current.messages).toHaveLength(0);
  });

  it("handles non-ok HTTP response gracefully", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 404 })
    );
    const { result } = renderHook(() => useSystemMessages());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).not.toBeNull();
    expect(result.current.messages).toHaveLength(0);
  });

  it("markRead reduces unreadCount", async () => {
    const { result } = renderHook(() => useSystemMessages());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => result.current.markRead("msg-1"));
    expect(result.current.unreadCount).toBe(1);
  });

  it("dismiss removes message from visible list", async () => {
    const { result } = renderHook(() => useSystemMessages());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => result.current.dismiss("msg-1"));
    expect(result.current.messages.find((m) => m.id === "msg-1")).toBeUndefined();
    expect(result.current.messages).toHaveLength(1);
  });

  it("dismissAll removes all messages", async () => {
    const { result } = renderHook(() => useSystemMessages());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => result.current.dismissAll());
    expect(result.current.messages).toHaveLength(0);
    expect(result.current.unreadCount).toBe(0);
  });

  it("persists dismissed state to localStorage", async () => {
    const { result } = renderHook(() => useSystemMessages());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => result.current.dismiss("msg-1"));

    const stored = JSON.parse(
      localStorage.getItem("stellarroute-system-messages-state") ?? "{}"
    );
    expect(stored.dismissed).toContain("msg-1");
  });

  it("restores dismissed state from localStorage on mount", async () => {
    localStorage.setItem(
      "stellarroute-system-messages-state",
      JSON.stringify({ read: ["msg-1"], dismissed: ["msg-1"] })
    );

    const { result } = renderHook(() => useSystemMessages());
    await waitFor(() => expect(result.current.loading).toBe(false));

    // msg-1 was previously dismissed — should not appear
    expect(result.current.messages.find((m) => m.id === "msg-1")).toBeUndefined();
    expect(result.current.messages).toHaveLength(1);
  });
});

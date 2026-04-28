"use client";

import { useCallback, useEffect, useState } from "react";

export type MessageSeverity = "info" | "warning" | "error" | "maintenance";

export interface SystemMessage {
  id: string;
  title: string;
  body: string;
  severity: MessageSeverity;
  /** ISO-8601 timestamp */
  created_at: string;
}

interface MessageState {
  read: string[];
  dismissed: string[];
}

const STORAGE_KEY = "stellarroute-system-messages-state";
const MESSAGES_URL =
  process.env.NEXT_PUBLIC_MESSAGES_URL ?? "/api/system-messages.json";
const POLL_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

function loadState(): MessageState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as MessageState;
  } catch {
    // ignore
  }
  return { read: [], dismissed: [] };
}

function saveState(state: MessageState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // ignore
  }
}

export interface UseSystemMessagesResult {
  messages: SystemMessage[];
  unreadCount: number;
  loading: boolean;
  error: Error | null;
  markRead: (id: string) => void;
  dismiss: (id: string) => void;
  dismissAll: () => void;
}

export function useSystemMessages(): UseSystemMessagesResult {
  const [allMessages, setAllMessages] = useState<SystemMessage[]>([]);
  const [state, setState] = useState<MessageState>({ read: [], dismissed: [] });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  // Load persisted read/dismissed state once on mount
  useEffect(() => {
    setState(loadState());
  }, []);

  const fetchMessages = useCallback(async (signal?: AbortSignal) => {
    try {
      const res = await fetch(MESSAGES_URL, { signal });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = (await res.json()) as SystemMessage[];
      setAllMessages(data);
      setError(null);
    } catch (err) {
      if (err instanceof Error && err.name === "AbortError") return;
      // Fetch failure must not block core swap — just surface the error
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, []);

  // Initial fetch + polling
  useEffect(() => {
    const controller = new AbortController();
    fetchMessages(controller.signal);
    const id = setInterval(
      () => fetchMessages(controller.signal),
      POLL_INTERVAL_MS
    );
    return () => {
      controller.abort();
      clearInterval(id);
    };
  }, [fetchMessages]);

  const markRead = useCallback((id: string) => {
    setState((prev) => {
      if (prev.read.includes(id)) return prev;
      const next = { ...prev, read: [...prev.read, id] };
      saveState(next);
      return next;
    });
  }, []);

  const dismiss = useCallback((id: string) => {
    setState((prev) => {
      if (prev.dismissed.includes(id)) return prev;
      const next = {
        read: prev.read.includes(id) ? prev.read : [...prev.read, id],
        dismissed: [...prev.dismissed, id],
      };
      saveState(next);
      return next;
    });
  }, []);

  const dismissAll = useCallback(() => {
    setState((prev) => {
      const allIds = allMessages.map((m) => m.id);
      const next: MessageState = {
        read: Array.from(new Set([...prev.read, ...allIds])),
        dismissed: Array.from(new Set([...prev.dismissed, ...allIds])),
      };
      saveState(next);
      return next;
    });
  }, [allMessages]);

  // Visible = not dismissed
  const messages = allMessages.filter((m) => !state.dismissed.includes(m.id));
  const unreadCount = messages.filter((m) => !state.read.includes(m.id)).length;

  return {
    messages,
    unreadCount,
    loading,
    error,
    markRead,
    dismiss,
    dismissAll,
  };
}

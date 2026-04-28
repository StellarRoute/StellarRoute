"use client";

import { useEffect, useRef, useState } from "react";
import { Bell, X, CheckCheck } from "lucide-react";
import {
  useSystemMessages,
  type MessageSeverity,
} from "@/hooks/useSystemMessages";

const SEVERITY_STYLES: Record<MessageSeverity, string> = {
  info: "border-l-blue-500 bg-blue-50 dark:bg-blue-950/30",
  warning: "border-l-yellow-500 bg-yellow-50 dark:bg-yellow-950/30",
  error: "border-l-red-500 bg-red-50 dark:bg-red-950/30",
  maintenance: "border-l-purple-500 bg-purple-50 dark:bg-purple-950/30",
};

const SEVERITY_BADGE: Record<MessageSeverity, string> = {
  info: "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
  warning:
    "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
  error: "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200",
  maintenance:
    "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200",
};

export function NotificationInbox() {
  const { messages, unreadCount, loading, error, markRead, dismiss, dismissAll } =
    useSystemMessages();
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  // Mark all visible messages as read when panel opens
  useEffect(() => {
    if (open) {
      messages.forEach((m) => markRead(m.id));
    }
  }, [open, messages, markRead]);

  return (
    <div className="relative" ref={panelRef}>
      {/* Bell button */}
      <button
        aria-label={`Notifications${unreadCount > 0 ? `, ${unreadCount} unread` : ""}`}
        aria-expanded={open}
        aria-haspopup="dialog"
        onClick={() => setOpen((v) => !v)}
        className="relative flex items-center justify-center rounded-md p-2 text-sm font-medium hover:bg-accent hover:text-accent-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <Bell className="h-4 w-4" />
        {unreadCount > 0 && (
          <span
            aria-hidden="true"
            className="absolute -top-0.5 -right-0.5 flex h-4 w-4 items-center justify-center rounded-full bg-red-500 text-[10px] font-bold text-white"
          >
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        )}
      </button>

      {/* Dropdown panel */}
      {open && (
        <div
          role="dialog"
          aria-label="System notifications"
          className="absolute right-0 top-full mt-2 z-50 w-80 rounded-lg border bg-background shadow-lg"
        >
          {/* Header */}
          <div className="flex items-center justify-between border-b px-4 py-3">
            <span className="text-sm font-semibold">Notifications</span>
            {messages.length > 0 && (
              <button
                onClick={dismissAll}
                className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                aria-label="Dismiss all notifications"
              >
                <CheckCheck className="h-3 w-3" />
                Dismiss all
              </button>
            )}
          </div>

          {/* Body */}
          <div className="max-h-96 overflow-y-auto">
            {loading && (
              <p className="px-4 py-6 text-center text-sm text-muted-foreground">
                Loading…
              </p>
            )}

            {!loading && error && (
              <p className="px-4 py-6 text-center text-sm text-muted-foreground">
                Could not load notifications.
              </p>
            )}

            {!loading && !error && messages.length === 0 && (
              <p className="px-4 py-6 text-center text-sm text-muted-foreground">
                No notifications
              </p>
            )}

            {!loading &&
              messages.map((msg) => (
                <div
                  key={msg.id}
                  className={`border-l-4 px-4 py-3 ${SEVERITY_STYLES[msg.severity]} relative`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span
                          className={`inline-block rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${SEVERITY_BADGE[msg.severity]}`}
                        >
                          {msg.severity}
                        </span>
                        <span className="text-xs text-muted-foreground truncate">
                          {new Date(msg.created_at).toLocaleDateString()}
                        </span>
                      </div>
                      <p className="text-sm font-medium leading-snug">
                        {msg.title}
                      </p>
                      <p className="mt-0.5 text-xs text-muted-foreground leading-relaxed">
                        {msg.body}
                      </p>
                    </div>
                    <button
                      onClick={() => dismiss(msg.id)}
                      aria-label={`Dismiss: ${msg.title}`}
                      className="shrink-0 rounded p-0.5 text-muted-foreground hover:text-foreground transition-colors"
                    >
                      <X className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
}

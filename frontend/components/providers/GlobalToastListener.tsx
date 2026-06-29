"use client";

import { useEffect } from "react";
import { toast } from "sonner";
import { StellarRouteApiError } from "@/lib/api/client";

/**
 * GlobalToastListener listens for unhandled errors and promise rejections
 * across the application and displays them as beautiful toast notifications.
 */
export function GlobalToastListener() {
  useEffect(() => {
    const recentlyShown = new Map<string, number>();
    const DEDUPE_WINDOW_MS = 4000;
    const shouldShowToast = (key: string) => {
      const now = Date.now();
      const lastShown = recentlyShown.get(key) ?? 0;
      if (now - lastShown < DEDUPE_WINDOW_MS) {
        return false;
      }
      recentlyShown.set(key, now);
      return true;
    };

    const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
      const error = event.reason;
      
      // Ignore aborted requests
      if (error instanceof Error && error.name === 'AbortError') {
        return;
      }

      console.error("Unhandled Promise Rejection:", error);

      if (error instanceof StellarRouteApiError) {
        if (!shouldShowToast(`api:${error.code}`)) {
          return;
        }
        toast.error("API Error", {
          description: error.message,
          duration: 5000,
        });
      } else if (error instanceof Error) {
        if (!shouldShowToast(`error:${error.message}`)) {
          return;
        }
        toast.error("An unexpected error occurred", {
          description: error.message,
          duration: 5000,
        });
      } else {
        toast.error("An unknown error occurred", {
          duration: 5000,
        });
      }
    };

    const handleWindowError = (event: ErrorEvent) => {
      // Ignore resize-observer errors (common in development/certain libraries)
      if (event.message === 'ResizeObserver loop limit exceeded' || 
          event.message === 'ResizeObserver loop completed with undelivered notifications.') {
        return;
      }

      console.error("Global Window Error:", event.error);

      const dedupeKey = event.error instanceof StellarRouteApiError
        ? `api:${event.error.code}`
        : `runtime:${event.message}`;
      if (!shouldShowToast(dedupeKey)) {
        return;
      }

      toast.error("Runtime Error", {
        description: event.message || "A runtime error occurred in the browser.",
        duration: 5000,
      });
    };

    window.addEventListener("unhandledrejection", handleUnhandledRejection);
    window.addEventListener("error", handleWindowError);

    return () => {
      window.removeEventListener("unhandledrejection", handleUnhandledRejection);
      window.removeEventListener("error", handleWindowError);
    };
  }, []);

  return null;
}

import { AlertTriangle, Inbox, Loader2, RefreshCw } from "lucide-react";
import { type ReactNode, useMemo } from "react";
import { Button } from "@/components/ui/button";

type ViewStateVariant = "loading" | "empty" | "error";

interface ViewStateProps {
  variant: ViewStateVariant;
  title: string;
  description: string;
  action?: ReactNode;
  className?: string;
}

const iconByVariant: Record<ViewStateVariant, ReactNode> = {
  loading: <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" aria-hidden="true" />,
  empty: <Inbox className="h-6 w-6 text-muted-foreground" aria-hidden="true" />,
  error: <AlertTriangle className="h-6 w-6 text-destructive" aria-hidden="true" />,
};

export function ViewState({
  variant,
  title,
  description,
  action,
  className,
}: ViewStateProps) {
  const role = variant === "error" ? "alert" : "status";

  return (
    <div
      role={role}
      className={`flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed p-6 text-center ${className ?? ""}`}
    >
      {iconByVariant[variant]}
      <div className="space-y-1">
        <h3 className="text-sm font-semibold">{title}</h3>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      {action ? <div>{action}</div> : null}
    </div>
  );
}

// ─── Composable loading state ──────────────────────────────────────

interface LoadingStateProps {
  message?: string;
  className?: string;
}

export function LoadingState({ message = "Loading...", className }: LoadingStateProps) {
  return (
    <ViewState
      variant="loading"
      title={message}
      description=""
      className={className}
    />
  );
}

// ─── Error state with optional retry ────────────────────────────────

interface ErrorStateProps {
  message: string;
  onRetry?: () => void;
  className?: string;
}

export function ErrorState({ message, onRetry, className }: ErrorStateProps) {
  return (
    <ViewState
      variant="error"
      title="Something went wrong"
      description={message}
      action={
        onRetry ? (
          <Button variant="outline" size="sm" onClick={onRetry}>
            <RefreshCw className="mr-1.5 h-3.5 w-3.5" />
            Retry
          </Button>
        ) : undefined
      }
      className={className}
    />
  );
}

// ─── Empty state ────────────────────────────────────────────────────

interface EmptyStateProps {
  message?: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({
  message = "No data",
  description = "There is nothing to display yet.",
  action,
  className,
}: EmptyStateProps) {
  return (
    <ViewState
      variant="empty"
      title={message}
      description={description}
      action={action}
      className={className}
    />
  );
}

// ─── useViewState hook ──────────────────────────────────────────────

type ViewStateResult<T> =
  | { state: "loading"; component: ReactNode }
  | { state: "error"; component: ReactNode }
  | { state: "empty"; component: ReactNode }
  | { state: "ready"; data: T };

/**
 * Standard hook to determine which view state to render.
 *
 * Returns a discriminated union — check `.state` to branch.
 * For `"ready"`, use `.data` to render the actual content.
 *
 * @example
 * const view = useViewState(data, isLoading, error);
 * if (view.state !== "ready") return view.component;
 * return <ActualContent data={view.data} />;
 */
export function useViewState<T>(
  data: T | null | undefined,
  isLoading: boolean,
  error: string | null | undefined,
  options?: {
    loadingMessage?: string;
    emptyMessage?: string;
    emptyDescription?: string;
    emptyAction?: ReactNode;
    onRetry?: () => void;
  },
): ViewStateResult<NonNullable<T>> {
  return useMemo(() => {
    if (isLoading) {
      return {
        state: "loading" as const,
        component: <LoadingState message={options?.loadingMessage} />,
      };
    }

    if (error) {
      return {
        state: "error" as const,
        component: <ErrorState message={error} onRetry={options?.onRetry} />,
      };
    }

    if (data === null || data === undefined) {
      return {
        state: "empty" as const,
        component: (
          <EmptyState
            message={options?.emptyMessage}
            description={options?.emptyDescription}
            action={options?.emptyAction}
          />
        ),
      };
    }

    return {
      state: "ready" as const,
      data: data as NonNullable<T>,
    };
  }, [data, isLoading, error, options?.loadingMessage, options?.emptyMessage, options?.emptyDescription, options?.emptyAction, options?.onRetry]);
}

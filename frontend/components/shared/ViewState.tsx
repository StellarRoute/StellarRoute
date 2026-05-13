import {
  AlertTriangle,
  History,
  Inbox,
  Loader2,
  Route,
  Wallet,
  WandSparkles,
} from "lucide-react";
import { ReactNode } from "react";

export type ViewStateVariant = "loading" | "empty" | "error";
export type SwapViewStateKind = "quote" | "routes" | "history" | "wallet";

interface ViewStateProps {
  variant: ViewStateVariant;
  title: string;
  description: string;
  action?: ReactNode;
  className?: string;
  icon?: ReactNode;
}

const iconByVariant: Record<ViewStateVariant, ReactNode> = {
  loading: <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" aria-hidden="true" />,
  empty: <Inbox className="h-6 w-6 text-muted-foreground" aria-hidden="true" />,
  error: <AlertTriangle className="h-6 w-6 text-destructive" aria-hidden="true" />,
};

const iconByKind: Record<SwapViewStateKind, ReactNode> = {
  quote: <WandSparkles className="h-6 w-6 text-muted-foreground" aria-hidden="true" />,
  routes: <Route className="h-6 w-6 text-muted-foreground" aria-hidden="true" />,
  history: <History className="h-6 w-6 text-muted-foreground" aria-hidden="true" />,
  wallet: <Wallet className="h-6 w-6 text-muted-foreground" aria-hidden="true" />,
};

const copyByKind: Record<
  SwapViewStateKind,
  Record<ViewStateVariant, { title: string; description: string }>
> = {
  quote: {
    loading: {
      title: "Loading quote",
      description: "Fetching the latest executable price for this pair.",
    },
    empty: {
      title: "No quote yet",
      description: "Choose a pair and amount to preview an executable swap quote.",
    },
    error: {
      title: "Quote unavailable",
      description: "The quote request failed. Check the pair or retry in a moment.",
    },
  },
  routes: {
    loading: {
      title: "Loading routes",
      description: "Comparing available venues and route candidates.",
    },
    empty: {
      title: "No route data",
      description: "Route details appear once a quote returns at least one path.",
    },
    error: {
      title: "Route unavailable",
      description: "Route data could not be loaded for the selected pair.",
    },
  },
  history: {
    loading: {
      title: "Loading activity",
      description: "Preparing your recent swap activity.",
    },
    empty: {
      title: "No Transactions Found",
      description: "You have not made any swaps yet, or your filters are too restrictive.",
    },
    error: {
      title: "History unavailable",
      description: "Transaction activity could not be loaded.",
    },
  },
  wallet: {
    loading: {
      title: "Checking wallet",
      description: "Reading wallet status and permissions.",
    },
    empty: {
      title: "Wallet not connected",
      description: "Connect a wallet to unlock balances and swap execution.",
    },
    error: {
      title: "Wallet unavailable",
      description: "Wallet status could not be verified.",
    },
  },
};

export function ViewState({
  variant,
  title,
  description,
  action,
  className,
  icon,
}: ViewStateProps) {
  const role = variant === "error" ? "alert" : "status";

  return (
    <div
      role={role}
      aria-busy={variant === "loading" || undefined}
      className={`flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed p-6 text-center ${className ?? ""}`}
    >
      {icon ?? iconByVariant[variant]}
      <div className="space-y-1">
        <h3 className="text-sm font-semibold">{title}</h3>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      {action ? <div>{action}</div> : null}
    </div>
  );
}

export function SwapViewState({
  kind,
  variant,
  action,
  className,
  title,
  description,
}: {
  kind: SwapViewStateKind;
  variant: ViewStateVariant;
  action?: ReactNode;
  className?: string;
  title?: string;
  description?: string;
}) {
  const copy = copyByKind[kind][variant];

  return (
    <ViewState
      variant={variant}
      title={title ?? copy.title}
      description={description ?? copy.description}
      action={action}
      className={className}
      icon={variant === "loading" || variant === "error" ? undefined : iconByKind[kind]}
    />
  );
}

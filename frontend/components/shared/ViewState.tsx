import { AlertTriangle, Inbox, Loader2 } from "lucide-react";
import { ReactNode } from "react";

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

import { Loader2 } from "lucide-react";

import { cn } from "@/lib/utils";
import { useReducedMotion } from "@/hooks/useReducedMotion";

interface SpinnerProps {
  className?: string;
  /** Accessible label announced by screen readers. Defaults to "Loading". */
  label?: string;
}

/**
 * Shared loading indicator. Spins unless the user prefers reduced motion,
 * in which case it renders statically to avoid triggering vestibular
 * discomfort while still communicating a busy state visually and via ARIA.
 */
export function Spinner({ className, label = "Loading" }: SpinnerProps) {
  const prefersReducedMotion = useReducedMotion();

  return (
    <Loader2
      role="status"
      aria-label={label}
      className={cn("h-5 w-5", !prefersReducedMotion && "animate-spin", className)}
    />
  );
}

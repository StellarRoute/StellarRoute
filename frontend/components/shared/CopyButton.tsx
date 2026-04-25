"use client";

import { Copy, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { cn } from "@/lib/utils";

interface CopyButtonProps {
  value: string;
  /** Accessible label describing what is being copied */
  label?: string;
  className?: string;
}

export function CopyButton({
  value,
  label = "Copy to clipboard",
  className,
}: CopyButtonProps) {
  const { copy, copied } = useCopyToClipboard();

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className={cn("h-6 w-6 shrink-0", className)}
      onClick={() => copy(value)}
      aria-label={copied ? "Copied!" : label}
    >
      {copied ? (
        <Check className="h-3 w-3 text-success" aria-hidden="true" />
      ) : (
        <Copy className="h-3 w-3" aria-hidden="true" />
      )}
    </Button>
  );
}

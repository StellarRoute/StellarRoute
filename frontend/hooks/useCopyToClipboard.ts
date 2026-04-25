import { useState, useCallback } from "react";
import { toast } from "sonner";

export function useCopyToClipboard() {
  const [copied, setCopied] = useState(false);

  const copy = useCallback(async (value: string) => {
    if (!navigator?.clipboard) {
      toast.error("Clipboard not available in this browser");
      return false;
    }
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      toast.success("Copied to clipboard");
      setTimeout(() => setCopied(false), 2000);
      return true;
    } catch {
      toast.error("Failed to copy to clipboard");
      return false;
    }
  }, []);

  return { copy, copied };
}

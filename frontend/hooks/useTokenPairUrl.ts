"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter, useSearchParams, usePathname } from "next/navigation";

export interface UseTokenPairUrlResult {
  /** Currently selected base asset from URL */
  base: string | undefined;
  /** Currently selected quote asset from URL */
  quote: string | undefined;
  /** Update both base and quote in URL */
  setPair: (base: string, quote: string) => void;
  /** Whether the URL state is being initialized */
  isInitializing: boolean;
}

/**
 * Hook to manage token pair selection in URL query parameters.
 * 
 * Syncs base and quote assets with URL params (?base=native&quote=USDC:ISSUER)
 * so that refresh and back/forward navigation work correctly.
 * 
 * @example
 * ```tsx
 * const { base, quote, setPair } = useTokenPairUrl();
 * 
 * <TokenPairSelector
 *   selectedBase={base}
 *   selectedQuote={quote}
 *   onPairChange={setPair}
 * />
 * ```
 */
export function useTokenPairUrl(): UseTokenPairUrlResult {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const [isInitializing] = useState(false);

  const base = searchParams.get("base") || undefined;
  const quote = searchParams.get("quote") || undefined;

  const setPair = useCallback(
    (newBase: string, newQuote: string) => {
      const params = new URLSearchParams(searchParams.toString());

      if (newBase) {
        params.set("base", newBase);
      } else {
        params.delete("base");
      }

      if (newQuote) {
        params.set("quote", newQuote);
      } else {
        params.delete("quote");
      }

      const queryString = params.toString();
      const url = queryString ? `${pathname}?${queryString}` : pathname;

      router.push(url, { scroll: false });
    },
    [pathname, router, searchParams]
  );

  return {
    base,
    quote,
    setPair,
    isInitializing,
  };
}

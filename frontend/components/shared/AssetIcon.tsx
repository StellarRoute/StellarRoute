"use client";

import { useEffect, useMemo, useState } from "react";

import { cn } from "@/lib/utils";

export interface AssetIconProps {
  symbol: string;
  src?: string;
  alt?: string;
  className?: string;
  imageClassName?: string;
  fallbackClassName?: string;
  maxCharacters?: number;
}

function getFallbackLabel(symbol: string, maxCharacters: number): string {
  const normalized = symbol.trim().toUpperCase();
  if (!normalized) {
    return "?";
  }

  if (normalized.length <= maxCharacters) {
    return normalized;
  }

  return normalized.slice(0, Math.max(1, maxCharacters));
}

export function AssetIcon({
  symbol,
  src,
  alt,
  className,
  imageClassName,
  fallbackClassName,
  maxCharacters = 2,
}: AssetIconProps) {
  const [imageFailed, setImageFailed] = useState(false);

  useEffect(() => {
    setImageFailed(false);
  }, [src]);

  const fallbackLabel = useMemo(
    () => getFallbackLabel(symbol, maxCharacters),
    [maxCharacters, symbol]
  );
  const showImage = Boolean(src) && !imageFailed;
  const imageAlt = alt ?? `${symbol} icon`;

  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full border border-border/60 bg-muted text-[0.65rem] font-semibold uppercase text-foreground/80",
        className
      )}
      aria-hidden="true"
    >
      {showImage ? (
        <img
          src={src}
          alt={imageAlt}
          className={cn("h-full w-full object-cover", imageClassName)}
          onError={() => setImageFailed(true)}
        />
      ) : (
        <span className={cn("leading-none", fallbackClassName)}>
          {fallbackLabel}
        </span>
      )}
    </span>
  );
}

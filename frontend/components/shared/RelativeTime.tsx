'use client';

import { useMemo } from 'react';
import { formatDistanceToNow, format } from 'date-fns';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';

interface RelativeTimeProps {
  /**
   * Timestamp in milliseconds or Date object
   */
  timestamp: number | Date;
  
  /**
   * Whether to add "ago" suffix (e.g., "2 hours ago" vs "2 hours")
   * @default true
   */
  addSuffix?: boolean;
  
  /**
   * Format string for the absolute time tooltip
   * @default "PPpp" (e.g., "Apr 29, 2021, 12:00:00 PM")
   */
  absoluteFormat?: string;
  
  /**
   * Additional CSS classes for the wrapper
   */
  className?: string;
  
  /**
   * Whether to show the tooltip
   * @default true
   */
  showTooltip?: boolean;
}

/**
 * RelativeTime component displays a human-readable relative timestamp
 * (e.g., "2 hours ago") with an absolute timestamp tooltip on hover.
 * 
 * @example
 * ```tsx
 * <RelativeTime timestamp={Date.now() - 3600000} />
 * // Displays: "1 hour ago"
 * // Tooltip: "May 26, 2026, 6:09:34 PM"
 * ```
 */
export function RelativeTime({
  timestamp,
  addSuffix = true,
  absoluteFormat = 'PPpp',
  className = '',
  showTooltip = true,
}: RelativeTimeProps) {
  const date = useMemo(() => {
    return timestamp instanceof Date ? timestamp : new Date(timestamp);
  }, [timestamp]);

  const relativeTime = useMemo(() => {
    return formatDistanceToNow(date, { addSuffix });
  }, [date, addSuffix]);

  const absoluteTime = useMemo(() => {
    return format(date, absoluteFormat);
  }, [date, absoluteFormat]);

  if (!showTooltip) {
    return (
      <span className={className} data-testid="relative-time">
        {relativeTime}
      </span>
    );
  }

  return (
    <TooltipProvider delayDuration={300}>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            className={`cursor-help underline decoration-dotted decoration-muted-foreground/40 ${className}`}
            data-testid="relative-time"
          >
            {relativeTime}
          </span>
        </TooltipTrigger>
        <TooltipContent>
          <p className="text-xs font-medium" data-testid="absolute-time">
            {absoluteTime}
          </p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

'use client';

import { useEffect, useRef, useState } from 'react';

interface ProgressiveLoadingOptions {
  minimumSkeletonMs?: number;
  enterDurationMs?: number;
}

interface ProgressiveLoadingState {
  showSkeleton: boolean;
  contentClassName: string;
}

const DEFAULT_MINIMUM_SKELETON_MS = 260;
const DEFAULT_ENTER_DURATION_MS = 320;

export function useProgressiveLoadingTransition(
  isLoading: boolean,
  {
    minimumSkeletonMs = DEFAULT_MINIMUM_SKELETON_MS,
    enterDurationMs = DEFAULT_ENTER_DURATION_MS,
  }: ProgressiveLoadingOptions = {},
): ProgressiveLoadingState {
  const [showSkeleton, setShowSkeleton] = useState(isLoading);
  const [isEntering, setIsEntering] = useState(!isLoading);

  const loadingStartedAtRef = useRef<number | null>(isLoading ? Date.now() : null);
  const revealTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const enterTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (revealTimerRef.current) {
      clearTimeout(revealTimerRef.current);
      revealTimerRef.current = null;
    }
    if (enterTimerRef.current) {
      clearTimeout(enterTimerRef.current);
      enterTimerRef.current = null;
    }

    if (isLoading) {
      loadingStartedAtRef.current = Date.now();
      setShowSkeleton(true);
      setIsEntering(false);
      return;
    }

    if (!showSkeleton) {
      setIsEntering(true);
      enterTimerRef.current = setTimeout(() => {
        setIsEntering(false);
      }, enterDurationMs);
      return;
    }

    const loadingStartedAt = loadingStartedAtRef.current ?? Date.now();
    const elapsed = Date.now() - loadingStartedAt;
    const waitMs = Math.max(minimumSkeletonMs - elapsed, 0);

    revealTimerRef.current = setTimeout(() => {
      setShowSkeleton(false);
      setIsEntering(true);
      enterTimerRef.current = setTimeout(() => {
        setIsEntering(false);
      }, enterDurationMs);
    }, waitMs);

    return () => {
      if (revealTimerRef.current) {
        clearTimeout(revealTimerRef.current);
        revealTimerRef.current = null;
      }
      if (enterTimerRef.current) {
        clearTimeout(enterTimerRef.current);
        enterTimerRef.current = null;
      }
    };
  }, [isLoading, showSkeleton, minimumSkeletonMs, enterDurationMs]);

  return {
    showSkeleton,
    contentClassName: isEntering ? 'animate-in fade-in slide-in-from-bottom-1 duration-300' : '',
  };
}

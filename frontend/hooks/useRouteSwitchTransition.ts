'use client';

import { useEffect, useRef, useState } from 'react';
import { useMotionAnimateIn, MOTION_DURATION } from '@/lib/motion';

interface RouteSwitchTransitionOptions {
  durationMs?: number;
}

interface RouteSwitchTransitionState {
  isSwitching: boolean;
  animateInClass: string;
}

/**
 * Hook to handle route switch animations.
 * Adds a transition effect when switching between different routes.
 *
 * @param routeKey - A key that changes when the route updates (e.g., route ID or hash)
 * @param options - Configuration options
 * @returns State and classes for the switch animation
 */
export function useRouteSwitchTransition(
  routeKey: string | number | undefined,
  { durationMs = MOTION_DURATION.ROUTE_SWITCH }: RouteSwitchTransitionOptions = {},
): RouteSwitchTransitionState {
  const [isSwitching, setIsSwitching] = useState(false);
  const previousKeyRef = useRef(routeKey);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const animateInClass = useMotionAnimateIn(
    'fade-in slide-in-from-right-2',
    `duration-${Math.round(durationMs / 10) * 10}`,
  );

  useEffect(() => {
    if (routeKey !== previousKeyRef.current && previousKeyRef.current !== undefined) {
      setIsSwitching(true);
      
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      
      timerRef.current = setTimeout(() => {
        setIsSwitching(false);
      }, durationMs);
    }
    
    previousKeyRef.current = routeKey;

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [routeKey, durationMs, animateInClass]);

  return {
    isSwitching,
    animateInClass: isSwitching ? animateInClass : '',
  };
}

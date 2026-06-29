'use client';

import { useReducedMotion } from '@/hooks/useReducedMotion';

/**
 * Motion Design System for StellarRoute
 *
 * Defines standardized durations, easing functions, and utilities
 * for consistent animations across the application.
 *
 * All animations respect the user's reduced-motion preference.
 */

// Motion Durations (in milliseconds)
export const MOTION_DURATION = {
  // Skeleton exit animation (fade out skeleton, fade in content)
  SKELETON_EXIT: 260,
  // Quote refresh animation (subtle value change)
  QUOTE_REFRESH: 200,
  // Route switch animation (transition between different routes)
  ROUTE_SWITCH: 320,
  // Micro-interactions (hover, focus, small state changes)
  MICRO: 150,
  // Standard transitions (modals, panels, etc.)
  STANDARD: 300,
  // Long animations (complex transitions)
  LONG: 500,
} as const;

// Easing Functions
export const MOTION_EASING = {
  // Standard ease-out for most animations
  DEFAULT: 'ease-out',
  // Ease-in-out for bidirectional transitions
  IN_OUT: 'ease-in-out',
  // Bounce for playful elements (use sparingly)
  BOUNCE: 'cubic-bezier(0.34, 1.56, 0.64, 1)',
} as const;

/**
 * Hook that returns motion-safe values based on user's reduced-motion preference.
 *
 * @param motionValue - The value to use when motion is allowed
 * @param reducedValue - The value to use when reduced-motion is enabled
 * @returns The appropriate value based on user preference
 */
export function useMotionValue<T>(motionValue: T, reducedValue: T): T {
  const prefersReducedMotion = useReducedMotion();
  return prefersReducedMotion ? reducedValue : motionValue;
}

/**
 * Returns duration in milliseconds, or 0 if reduced-motion is enabled.
 *
 * @param duration - The duration in milliseconds
 * @returns The duration, or 0 if reduced-motion is enabled
 */
export function useMotionDuration(duration: number): number {
  return useMotionValue(duration, 0);
}

/**
 * Creates a CSS transition string that respects reduced-motion preference.
 *
 * @param properties - The CSS properties to transition
 * @param duration - The duration in milliseconds
 * @param easing - The easing function
 * @returns A CSS transition string
 */
export function useMotionTransition(
  properties: string | string[],
  duration: number = MOTION_DURATION.MICRO,
  easing: string = MOTION_EASING.DEFAULT,
): string {
  const safeDuration = useMotionDuration(duration);
  const props = Array.isArray(properties) ? properties : [properties];

  if (safeDuration === 0) {
    return 'none';
  }

  return props.map((prop) => `${prop} ${safeDuration}ms ${easing}`).join(', ');
}

/**
 * Creates Tailwind CSS class names for animations that respect reduced-motion.
 *
 * @param animateInClass - The animate-in class (e.g., 'fade-in slide-in-from-bottom-1')
 * @param duration - The duration class (e.g., 'duration-300')
 * @returns The appropriate classes based on user preference
 */
export function useMotionAnimateIn(
  animateInClass: string,
  durationClass: string = 'duration-300',
): string {
  const prefersReducedMotion = useReducedMotion();
  return prefersReducedMotion ? '' : `${animateInClass} ${durationClass}`;
}

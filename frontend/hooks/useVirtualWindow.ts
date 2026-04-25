"use client";

import { RefObject, useEffect, useMemo, useState } from "react";

interface UseVirtualWindowOptions {
  containerRef: RefObject<HTMLElement | null>;
  itemCount: number;
  itemHeight: number;
  overscan?: number;
  enabled?: boolean;
  defaultViewportHeight?: number;
}

export interface VirtualWindowState {
  startIndex: number;
  endIndex: number;
  totalHeight: number;
  topSpacerHeight: number;
  bottomSpacerHeight: number;
  isVirtualized: boolean;
}

export function useVirtualWindow({
  containerRef,
  itemCount,
  itemHeight,
  overscan = 3,
  enabled = true,
  defaultViewportHeight = 320,
}: UseVirtualWindowOptions): VirtualWindowState {
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(defaultViewportHeight);

  useEffect(() => {
    if (!enabled) {
      setScrollTop(0);
      return;
    }

    const element = containerRef.current;
    if (!element) {
      return;
    }

    let frame = 0;

    const updateMeasurements = () => {
      setViewportHeight(element.clientHeight || defaultViewportHeight);
    };

    const handleScroll = () => {
      cancelAnimationFrame(frame);
      frame = window.requestAnimationFrame(() => {
        setScrollTop(element.scrollTop);
      });
    };

    updateMeasurements();
    element.addEventListener("scroll", handleScroll, { passive: true });
    window.addEventListener("resize", updateMeasurements);

    let resizeObserver: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      resizeObserver = new ResizeObserver(updateMeasurements);
      resizeObserver.observe(element);
    }

    return () => {
      cancelAnimationFrame(frame);
      element.removeEventListener("scroll", handleScroll);
      window.removeEventListener("resize", updateMeasurements);
      resizeObserver?.disconnect();
    };
  }, [containerRef, defaultViewportHeight, enabled]);

  return useMemo(() => {
    const totalHeight = itemCount * itemHeight;

    if (!enabled || itemCount === 0) {
      return {
        startIndex: 0,
        endIndex: itemCount,
        totalHeight,
        topSpacerHeight: 0,
        bottomSpacerHeight: 0,
        isVirtualized: false,
      };
    }

    const safeViewportHeight = Math.max(viewportHeight, defaultViewportHeight);
    const startIndex = Math.max(
      0,
      Math.floor(scrollTop / itemHeight) - overscan,
    );
    const endIndex = Math.min(
      itemCount,
      Math.ceil((scrollTop + safeViewportHeight) / itemHeight) + overscan,
    );
    const topSpacerHeight = startIndex * itemHeight;
    const bottomSpacerHeight = Math.max(
      0,
      totalHeight - topSpacerHeight - (endIndex - startIndex) * itemHeight,
    );

    return {
      startIndex,
      endIndex,
      totalHeight,
      topSpacerHeight,
      bottomSpacerHeight,
      isVirtualized: true,
    };
  }, [
    defaultViewportHeight,
    enabled,
    itemCount,
    itemHeight,
    overscan,
    scrollTop,
    viewportHeight,
  ]);
}

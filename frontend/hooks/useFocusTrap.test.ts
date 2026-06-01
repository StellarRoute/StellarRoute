// Feature: wallet-transaction-lifecycle, Property 7: Focus trap keeps Tab cycling within the modal

import { renderHook } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import * as fc from "fast-check";
import { useRef } from "react";
import { useFocusTrap } from "./useFocusTrap";

/**
 * Creates a container div with `count` focusable buttons appended to it,
 * attaches it to document.body, and returns the container along with a
 * cleanup function.
 */
function createFocusableContainer(count: number): {
  container: HTMLDivElement;
  buttons: HTMLButtonElement[];
  cleanup: () => void;
} {
  const container = document.createElement("div");
  const buttons: HTMLButtonElement[] = [];

  for (let i = 0; i < count; i++) {
    const btn = document.createElement("button");
    btn.textContent = `Button ${i}`;
    container.appendChild(btn);
    buttons.push(btn);
  }

  document.body.appendChild(container);

  return {
    container,
    buttons,
    cleanup: () => document.body.removeChild(container),
  };
}

/**
 * Dispatches a synthetic Tab keydown event on the document's active element
 * (or the container if nothing is focused) and returns the new active element.
 */
function pressTab(shiftKey = false): Element | null {
  const target = document.activeElement ?? document.body;
  const event = new KeyboardEvent("keydown", {
    key: "Tab",
    shiftKey,
    bubbles: true,
    cancelable: true,
  });
  target.dispatchEvent(event);
  return document.activeElement;
}

describe("useFocusTrap", () => {
  beforeEach(() => {
    // Reset focus to body before each test
    (document.body as HTMLElement).focus?.();
  });

  // -------------------------------------------------------------------------
  // Property 7: Focus trap keeps Tab cycling within the modal
  // -------------------------------------------------------------------------
  it("Property 7: Tab wraps from last to first focusable element", () => {
    // Feature: wallet-transaction-lifecycle, Property 7: Focus trap keeps Tab cycling within the modal
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 10 }), (count) => {
        const { container, buttons, cleanup } = createFocusableContainer(count);

        try {
          const containerRef = { current: container };

          const { unmount } = renderHook(() =>
            useFocusTrap(
              containerRef as React.RefObject<HTMLElement>,
              true
            )
          );

          // Focus the last button
          buttons[buttons.length - 1].focus();
          expect(document.activeElement).toBe(buttons[buttons.length - 1]);

          // Press Tab — should wrap to the first button
          pressTab(false);
          expect(document.activeElement).toBe(buttons[0]);

          unmount();
        } finally {
          cleanup();
        }
      }),
      { numRuns: 50 }
    );
  });

  it("Property 7: Shift+Tab wraps from first to last focusable element", () => {
    // Feature: wallet-transaction-lifecycle, Property 7: Focus trap keeps Tab cycling within the modal
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 10 }), (count) => {
        const { container, buttons, cleanup } = createFocusableContainer(count);

        try {
          const containerRef = { current: container };

          const { unmount } = renderHook(() =>
            useFocusTrap(
              containerRef as React.RefObject<HTMLElement>,
              true
            )
          );

          // Focus the first button
          buttons[0].focus();
          expect(document.activeElement).toBe(buttons[0]);

          // Press Shift+Tab — should wrap to the last button
          pressTab(true);
          expect(document.activeElement).toBe(buttons[buttons.length - 1]);

          unmount();
        } finally {
          cleanup();
        }
      }),
      { numRuns: 50 }
    );
  });

  it("Property 7: focus never escapes the container across multiple Tab presses", () => {
    // Feature: wallet-transaction-lifecycle, Property 7: Focus trap keeps Tab cycling within the modal
    fc.assert(
      fc.property(
        fc.integer({ min: 2, max: 10 }),
        fc.integer({ min: 1, max: 20 }),
        (count, tabPresses) => {
          const { container, buttons, cleanup } = createFocusableContainer(count);

          try {
            const containerRef = { current: container };

            const { unmount } = renderHook(() =>
              useFocusTrap(
                containerRef as React.RefObject<HTMLElement>,
                true
              )
            );

            // Start from the first button
            buttons[0].focus();

            for (let i = 0; i < tabPresses; i++) {
              pressTab(false);
              // After each Tab, focus must be on one of the container's buttons
              expect(buttons).toContain(document.activeElement);
            }

            unmount();
          } finally {
            cleanup();
          }
        }
      ),
      { numRuns: 30 }
    );
  });

  // -------------------------------------------------------------------------
  // Unit tests
  // -------------------------------------------------------------------------
  it("does not intercept Tab when active is false", () => {
    const { container, buttons, cleanup } = createFocusableContainer(2);

    try {
      const containerRef = { current: container };

      const { unmount } = renderHook(() =>
        useFocusTrap(
          containerRef as React.RefObject<HTMLElement>,
          false // trap is inactive
        )
      );

      // Focus the last button
      buttons[buttons.length - 1].focus();

      // Dispatch Tab — the trap should NOT intercept, so focus stays on last
      // (the browser would normally move focus out, but in jsdom it stays put
      // since there's no real tab order outside the container)
      const event = new KeyboardEvent("keydown", {
        key: "Tab",
        bubbles: true,
        cancelable: true,
      });
      buttons[buttons.length - 1].dispatchEvent(event);

      // Focus should NOT have been moved to the first button by the trap
      expect(document.activeElement).not.toBe(buttons[0]);

      unmount();
    } finally {
      cleanup();
    }
  });

  it("removes the listener on unmount", () => {
    const { container, buttons, cleanup } = createFocusableContainer(2);

    try {
      const containerRef = { current: container };

      const { unmount } = renderHook(() =>
        useFocusTrap(
          containerRef as React.RefObject<HTMLElement>,
          true
        )
      );

      unmount();

      // After unmount, Tab from the last button should NOT wrap to the first
      buttons[buttons.length - 1].focus();
      const event = new KeyboardEvent("keydown", {
        key: "Tab",
        bubbles: true,
        cancelable: true,
      });
      buttons[buttons.length - 1].dispatchEvent(event);

      // The trap listener is gone, so focus should not have been moved to first
      expect(document.activeElement).not.toBe(buttons[0]);
    } finally {
      cleanup();
    }
  });

  it("is a no-op when the container has no focusable elements", () => {
    const container = document.createElement("div");
    // No focusable children
    document.body.appendChild(container);

    try {
      const containerRef = { current: container };

      const { unmount } = renderHook(() =>
        useFocusTrap(
          containerRef as React.RefObject<HTMLElement>,
          true
        )
      );

      // Pressing Tab should not throw
      const event = new KeyboardEvent("keydown", {
        key: "Tab",
        bubbles: true,
        cancelable: true,
      });
      expect(() => container.dispatchEvent(event)).not.toThrow();

      unmount();
    } finally {
      document.body.removeChild(container);
    }
  });

  it("does not intercept non-Tab keys", () => {
    const { container, buttons, cleanup } = createFocusableContainer(2);

    try {
      const containerRef = { current: container };

      const { unmount } = renderHook(() =>
        useFocusTrap(
          containerRef as React.RefObject<HTMLElement>,
          true
        )
      );

      buttons[0].focus();

      // Press Enter — should not move focus
      const event = new KeyboardEvent("keydown", {
        key: "Enter",
        bubbles: true,
        cancelable: true,
      });
      buttons[0].dispatchEvent(event);

      expect(document.activeElement).toBe(buttons[0]);

      unmount();
    } finally {
      cleanup();
    }
  });
});

// Feature: wallet-transaction-lifecycle, Property 4: ExplorerLink URL is well-formed for any non-empty hash
import * as React from "react";
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import * as fc from "fast-check";
import { ExplorerLink } from "./ExplorerLink";

/**
 * Validates: Requirements 3.3, 3.6
 *
 * Property 4: ExplorerLink URL is well-formed for any non-empty hash
 */
describe("ExplorerLink", () => {
  it("Property 4: renders a well-formed anchor for any non-empty hash", () => {
    fc.assert(
      fc.property(
        fc.stringOf(fc.char(), { minLength: 1, maxLength: 64 }),
        (hash) => {
          const { unmount } = render(<ExplorerLink hash={hash} />);

          const link = screen.getByRole("link");

          expect(link).toHaveAttribute(
            "href",
            `https://stellar.expert/explorer/public/tx/${hash}`
          );
          expect(link).toHaveAttribute("target", "_blank");

          const rel = link.getAttribute("rel") ?? "";
          expect(rel).toContain("noreferrer");
          expect(rel).toContain("noopener");

          expect(link).toHaveAttribute(
            "aria-label",
            `View transaction ${hash.slice(0, 8)} on Stellar Expert`
          );

          unmount();
        }
      ),
      { numRuns: 100 }
    );
  });

  it("renders nothing when hash is an empty string", () => {
    const { container } = render(<ExplorerLink hash="" />);
    expect(container.firstChild).toBeNull();
  });

  it("renders nothing when hash is falsy (undefined cast to empty string)", () => {
    // Guard: passing empty string covers the falsy guard inside the component
    const { container } = render(<ExplorerLink hash="" />);
    expect(container.firstChild).toBeNull();
  });
});

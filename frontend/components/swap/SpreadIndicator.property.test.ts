import { describe, it, expect } from "vitest";
import fc from "fast-check";
import { getSpreadColor, getSpreadPercent } from "./SpreadIndicator";

describe("SpreadIndicator Property Tests", () => {
  it("spreadPercent matches (spreadBps / 100).toFixed(2) for any non-negative integer", () => {
    fc.assert(
      fc.property(fc.integer({ min: 0, max: 100000 }), (spreadBps) => {
        const percentStr = getSpreadPercent(spreadBps);
        const expected = (spreadBps / 100).toFixed(2);
        expect(percentStr).toBe(expected);
      })
    );
  });

  it("spreadColor maps to correct color classes based on defined boundaries", () => {
    fc.assert(
      fc.property(fc.integer({ min: 0, max: 10000 }), (spreadBps) => {
        const colorClass = getSpreadColor(spreadBps);
        if (spreadBps < 10) {
          expect(colorClass).toBe("text-emerald-500");
        } else if (spreadBps < 50) {
          expect(colorClass).toBe("text-blue-500");
        } else if (spreadBps < 200) {
          expect(colorClass).toBe("text-amber-500");
        } else {
          expect(colorClass).toBe("text-destructive");
        }
      })
    );
  });
});

import { describe, expect, it } from "vitest";

import { getNavItems } from "./nav-items";

describe("getNavItems", () => {
  it("includes analytics when the feature flag is enabled", () => {
    const items = getNavItems({ analyticsEnabled: true });
    expect(items.some((item) => item.href === "/analytics")).toBe(true);
  });

  it("omits analytics when the feature flag is disabled", () => {
    const items = getNavItems({ analyticsEnabled: false });
    expect(items.some((item) => item.href === "/analytics")).toBe(false);
  });
});

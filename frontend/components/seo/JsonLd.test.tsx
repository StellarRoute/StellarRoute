import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { JsonLd } from "./JsonLd";

describe("JsonLd", () => {
  it("renders a script tag with type application/ld+json", () => {
    const fixtureData = {
      "@context": "https://schema.org",
      "@type": "WebSite",
      name: "StellarRoute",
      url: "https://stellarroute.app",
    };

    const { container } = render(<JsonLd data={fixtureData} />);

    const scriptElement = container.querySelector("script");
    expect(scriptElement).not.toBeNull();
    expect(scriptElement?.getAttribute("type")).toBe("application/ld+json");

    const content = scriptElement?.innerHTML;
    expect(content).toBeTruthy();
    expect(JSON.parse(content!)).toEqual(fixtureData);
  });

  it("serializes an array fixture as application/ld+json", () => {
    const fixtureData = [
      { "@context": "https://schema.org", "@type": "Organization", name: "StellarRoute" },
      { "@context": "https://schema.org", "@type": "WebSite", name: "StellarRoute" },
    ];

    const { container } = render(<JsonLd data={fixtureData} />);
    const scriptElement = container.querySelector("script");
    expect(scriptElement?.getAttribute("type")).toBe("application/ld+json");
    expect(JSON.parse(scriptElement!.innerHTML)).toEqual(fixtureData);
  });
});

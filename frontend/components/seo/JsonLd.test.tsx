import { render } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { JsonLd, serializeJsonLd } from "./JsonLd";

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

  it("does not let a value close the script element", () => {
    const fixtureData = {
      "@context": "https://schema.org",
      "@type": "FAQPage",
      name: "</script><script>alert(1)</script>",
    };

    // Parse the server-rendered HTML the way a browser would, so a premature
    // </script> would show up as extra elements.
    const markup = renderToStaticMarkup(<JsonLd data={fixtureData} />);
    const doc = new DOMParser().parseFromString(
      `<!doctype html><html><head>${markup}</head><body></body></html>`,
      "text/html",
    );

    const scripts = doc.querySelectorAll("script");
    expect(scripts).toHaveLength(1);
    expect(markup).not.toContain("</script><script>");
    expect(JSON.parse(scripts[0].textContent!)).toEqual(fixtureData);
  });

  it("escapes <, >, &, U+2028 and U+2029 but keeps the data identical", () => {
    const fixtureData = {
      "@context": "https://schema.org",
      "@type": "Organization",
      name: "A & B <C> \u2028line\u2029para",
      url: "https://stellarroute.app/?a=1&b=2",
    };

    const serialized = serializeJsonLd(fixtureData);

    expect(serialized).not.toMatch(/[<>&\u2028\u2029]/);
    expect(serialized).toContain("\\u003cC\\u003e");
    expect(serialized).toContain("\\u0026");
    expect(JSON.parse(serialized)).toEqual(fixtureData);
  });

  it("matches JSON.stringify for data without special characters", () => {
    const fixtureData = {
      "@context": "https://schema.org",
      "@type": "WebSite",
      name: "StellarRoute",
      url: "https://stellarroute.app",
    };

    expect(serializeJsonLd(fixtureData)).toBe(JSON.stringify(fixtureData));
  });
});

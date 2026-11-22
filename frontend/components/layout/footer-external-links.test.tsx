/**
 * W12-FE-14 — Footer external links must carry target="_blank" + rel="noopener noreferrer".
 * Covers all three external links (GitHub, Stellar.org, Community) and the
 * "Built for Stellar" anchor; also verifies that internal links do NOT have
 * those attributes.
 */
import { render, screen, cleanup } from "@testing-library/react";
import { describe, it, expect, vi, afterEach } from "vitest";
import { Footer } from "./footer";

vi.mock("@/components/providers/wallet-provider", () => ({
  useWallet: () => ({ network: "testnet", setNetwork: vi.fn() }),
}));

vi.mock("@/lib/network-policy", () => ({
  getAllowedNetworks: vi.fn(() => ["testnet"]),
}));

vi.mock("next/link", () => ({
  default: ({
    children,
    href,
    ...props
  }: {
    children: React.ReactNode;
    href: string;
    [key: string]: unknown;
  }) => (
    <a href={href} {...props}>
      {children}
    </a>
  ),
}));

const EXTERNAL_LINK_NAMES = ["GitHub", "Stellar.org", "Community"];
const INTERNAL_LINK_NAMES = ["Swap", "Status", "Docs"];

describe("Footer – external link security attrs (W12-FE-14)", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it.each(EXTERNAL_LINK_NAMES)(
    '"%s" opens in a new tab with target="_blank"',
    (name) => {
      render(<Footer />);
      const link = screen.getByRole("link", { name: new RegExp(name, "i") });
      expect(link).toHaveAttribute("target", "_blank");
    },
  );

  it.each(EXTERNAL_LINK_NAMES)(
    '"%s" has rel="noopener noreferrer"',
    (name) => {
      render(<Footer />);
      const link = screen.getByRole("link", { name: new RegExp(name, "i") });
      expect(link).toHaveAttribute("rel", "noopener noreferrer");
    },
  );

  it("the Built-for-Stellar anchor opens in a new tab with security attrs", () => {
    render(<Footer />);
    // The branding paragraph contains a plain <a> pointing to stellar.org.
    // We pick it out by its exact text content.
    const links = screen.getAllByRole("link", { name: /stellar/i });
    const builtForLink = links.find((l) => l.textContent === "Stellar");
    expect(builtForLink).toBeDefined();
    expect(builtForLink).toHaveAttribute("target", "_blank");
    expect(builtForLink).toHaveAttribute("rel", "noopener noreferrer");
  });

  it.each(INTERNAL_LINK_NAMES)(
    '"%s" is an internal link without target="_blank"',
    (name) => {
      render(<Footer />);
      const link = screen.getByRole("link", { name: new RegExp(`^${name}$`, "i") });
      expect(link).not.toHaveAttribute("target", "_blank");
    },
  );

  it.each(INTERNAL_LINK_NAMES)(
    '"%s" is an internal link without rel="noopener noreferrer"',
    (name) => {
      render(<Footer />);
      const link = screen.getByRole("link", { name: new RegExp(`^${name}$`, "i") });
      expect(link).not.toHaveAttribute("rel", "noopener noreferrer");
    },
  );
});

import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import NotFound from "./not-found";

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

describe("NotFound page (404)", () => {
  it("renders the 404 heading", () => {
    render(<NotFound />);
    expect(screen.getByRole("heading", { name: "404" })).toBeInTheDocument();
  });

  it("renders a link back to /swap", () => {
    render(<NotFound />);
    const link = screen.getByRole("link", { name: /back to swap/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute("href", "/swap");
  });

  it("link carries the data-testid attribute", () => {
    render(<NotFound />);
    expect(screen.getByTestId("not-found-swap-link")).toBeInTheDocument();
  });

  it("renders a descriptive not-found message", () => {
    render(<NotFound />);
    expect(screen.getByText(/page not found/i)).toBeInTheDocument();
  });
});

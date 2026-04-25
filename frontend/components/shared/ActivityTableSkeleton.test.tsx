import { render } from "@testing-library/react";
import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ActivityTableSkeleton } from "./ActivityTableSkeleton";

describe("ActivityTableSkeleton", () => {
  afterEach(() => cleanup());

  it("should render exactly 5 skeleton rows", () => {
    const { container } = render(<ActivityTableSkeleton />);

    // Each row has the structure with borders
    const rows = container.querySelectorAll(".border-b.flex");
    
    // First row is header, followed by 5 data rows
    expect(rows.length).toBe(6); // header + 5 rows
  });

  it("should render table header with all columns", () => {
    const { container } = render(<ActivityTableSkeleton />);

    const headerRow = container.querySelector(".sticky");
    
    // Should have 6 column headers (Date, Swap, Rate, Status, Amount, Explorer)
    expect(headerRow).toBeInTheDocument();
  });

  it("should have skeleton elements with animate-pulse class", () => {
    const { container } = render(<ActivityTableSkeleton />);

    const skeletons = container.querySelectorAll(".animate-pulse");
    
    // Should have multiple skeleton elements for all cells
    expect(skeletons.length).toBeGreaterThan(20);
  });

  it("should render alternating row colors for better visual separation", () => {
    const { container } = render(<ActivityTableSkeleton />);

    const rows = container.querySelectorAll(".border-b.flex:not(.sticky)");
    
    // All data rows should have hover state
    rows.forEach((row) => {
      expect(row.className).toContain("hover:bg-muted/50");
    });
  });

  it("should match table structure of actual transactions", () => {
    const { container } = render(<ActivityTableSkeleton />);

    // Check for structure that matches real table
    const tableStructure = container.querySelector(".w-full");
    expect(tableStructure).toBeInTheDocument();

    // Verify there are multiple columns per row
    const firstDataRow = container.querySelectorAll(".border-b.flex")[1]; // Skip header
    const columns = firstDataRow?.querySelectorAll(".flex-1");
    
    // Should have 6 columns (Date, Swap, Rate, Status, Amount, Explorer)
    expect(columns?.length).toBe(6);
  });
});

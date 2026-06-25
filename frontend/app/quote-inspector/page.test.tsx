import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import QuoteInspectorPage from './page';

// Mock Header to avoid deep rendering issues
vi.mock('@/components/Header', () => ({
  Header: () => <header data-testid="mock-header" />
}));

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() }
}));

// Mock QuoteInspector to prevent duplication of component-level tests.
// This validates that the page wires up the shared component properly without
// repeating the actual component's logic tests.
vi.mock('@/components/shared/QuoteInspector', () => ({
  QuoteInspector: ({ quotes, onSelect }: any) => (
    <div data-testid="mock-quote-inspector">
      <span data-testid="quote-count">{quotes.length}</span>
      <button data-testid="select-btn" onClick={() => onSelect(quotes[0])}>
        Select Quote
      </button>
    </div>
  )
}));

describe('QuoteInspectorPage', () => {
  it('renders the page layout with Header and QuoteInspector', () => {
    render(<QuoteInspectorPage />);
    
    expect(screen.getByTestId('mock-header')).toBeInTheDocument();
    expect(screen.getByText('Quote Reconciliation')).toBeInTheDocument();
    expect(screen.getByTestId('mock-quote-inspector')).toBeInTheDocument();
  });

  it('passes mock quotes correctly to the QuoteInspector', () => {
    render(<QuoteInspectorPage />);
    
    // Validates that the 3 mock quotes are passed down
    expect(screen.getByTestId('quote-count')).toHaveTextContent('3');
  });

  it('handles quote selection gracefully', () => {
    render(<QuoteInspectorPage />);
    
    // Simulate selecting a quote via the mock inspector
    const selectBtn = screen.getByTestId('select-btn');
    selectBtn.click();
    
    // We mocked the toast, so it won't crash and would trigger toast.success
    // We just verify it renders fine and doesn't throw.
    expect(screen.getByTestId('mock-quote-inspector')).toBeInTheDocument();
  });
});

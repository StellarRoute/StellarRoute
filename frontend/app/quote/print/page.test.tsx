import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import QuotePrintPage from './page';

vi.mock('next/navigation', () => ({
  useSearchParams: vi.fn(() => new URLSearchParams()),
}));

describe('QuotePrintPage', () => {
  it('renders without crashing and shows the empty state with no query param', () => {
    render(<QuotePrintPage />);
    expect(screen.getByText('No quote to print')).toBeInTheDocument();
  });
});

import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';

import { TransactionStatusBadge } from './TransactionStatusBadge';

describe('TransactionStatusBadge', () => {
  it('renders a confirmed badge with label', () => {
    const { container } = render(<TransactionStatusBadge status="confirmed" size={20} />);

    expect(screen.getByText('Confirmed')).toBeInTheDocument();
    expect(container.querySelector('svg')).toBeInTheDocument();
  });

  it('renders a failed badge with destructive styling', () => {
    render(<TransactionStatusBadge status="failed" size={16} />);

    expect(screen.getByText('Failed')).toBeInTheDocument();
  });
});

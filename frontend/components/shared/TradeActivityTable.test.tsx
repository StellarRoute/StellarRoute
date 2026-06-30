// frontend/src/components/shared/TradeActivityTable.test.tsx
import React from 'react';
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TradeActivityTable } from './TradeActivityTable';
import { TradeRecord } from '../../types/trade';

const mockData: TradeRecord[] = [
  { id: '1', txHash: '123456789012345', timestamp: new Date(2026, 0, 1), action: 'BUY', amount: '100', asset: 'XLM' }
];

describe('TradeActivityTable component', () => {
  it('should render loading state when address is missing', () => {
    render(<TradeActivityTable initialData={mockData} />);
    expect(screen.getByTestId('loading-state')).toBeDefined();
  });

  it('should render empty state when data is empty', () => {
    render(<TradeActivityTable address="G..." initialData={[]} />);
    expect(screen.getByTestId('empty-state')).toBeDefined();
  });

  it('should render table rows cleanly', () => {
    render(<TradeActivityTable address="G..." initialData={mockData} />);
    expect(screen.getAllByTestId('trade-row').length).toBe(1);
    expect(screen.getByText('BUY')).toBeDefined();
  });
});
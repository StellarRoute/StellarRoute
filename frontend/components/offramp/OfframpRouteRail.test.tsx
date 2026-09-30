import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { OfframpRouteRail } from './OfframpRouteRail';
import type { OfframpRouteStep } from '@/lib/offramp/types';

describe('OfframpRouteRail', () => {
  const mockSteps: OfframpRouteStep[] = [
    {
      id: 'source',
      label: 'Send Asset',
      detail: 'Send from your wallet',
      active: true,
    },
    {
      id: 'payout',
      label: 'Receive Fiat',
      detail: 'Bank transfer',
      active: false,
    }
  ];

  it('renders all fixture steps correctly', () => {
    render(<OfframpRouteRail steps={mockSteps} />);

    // Acceptance criteria: Fixture steps render
    const routeRail = screen.getByTestId('offramp-route-rail');
    expect(routeRail).not.toBeNull();
    
    // Check that step labels and details are rendered
    expect(screen.getByText('Send Asset')).not.toBeNull();
    expect(screen.getByText('Send from your wallet')).not.toBeNull();
    expect(screen.getByText('Receive Fiat')).not.toBeNull();
    expect(screen.getByText('Bank transfer')).not.toBeNull();
    
    // Check that step numbers render correctly (index + 1)
    expect(screen.getByText('1')).not.toBeNull();
    expect(screen.getByText('2')).not.toBeNull();
  });
  
  it('renders empty list when no steps are provided', () => {
    render(<OfframpRouteRail steps={[]} />);
    const routeRail = screen.getByTestId('offramp-route-rail');
    expect(routeRail.children.length).toBe(0);
  });
});

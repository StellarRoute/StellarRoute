import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MobileRouteBottomSheet } from './MobileRouteBottomSheet';

// Force the media query hook to return false (simulating a mobile breakpoint layout)
vi.mock('@/hooks/useMediaQuery', () => ({
  useMediaQuery: () => false,
}));

// Mock the inner display graphic panel to isolate button actions
vi.mock('./RouteDisplay', () => {
  const MockComponent = () => (
    <div data-testid="route-display-mock">Mock Route Path</div>
  );
  return {
    default: MockComponent,
    RouteDisplay: MockComponent,
  };
});

describe('MobileRouteBottomSheet (Issue #501 Validation Specs)', () => {
  const defaultProps = {
    quote: null,
    amountOut: '100.00',
    isLoading: false,
  };

  it('should render the route summary CTA button on mobile breakpoints', () => {
    render(<MobileRouteBottomSheet {...defaultProps} />);
    expect(screen.getAllByTestId('route-sheet-trigger')[0]).toBeInTheDocument();
  });

  it('should open the bottom sheet panel on trigger click flow', async () => {
    render(<MobileRouteBottomSheet {...defaultProps} />);

    const trigger = screen.getAllByTestId('route-sheet-trigger')[0];
    fireEvent.click(trigger);

    expect(screen.getByTestId('route-sheet-content')).toBeInTheDocument();
    expect(screen.getByText('Select Order Route')).toBeInTheDocument();
    expect(screen.getByTestId('route-display-mock')).toBeInTheDocument();
  });

  it('should toggle between half and full height viewport snap positions matching gesture metrics', () => {
    render(<MobileRouteBottomSheet {...defaultProps} />);

    const trigger = screen.getAllByTestId('route-sheet-trigger')[0];
    fireEvent.click(trigger);

    const snapToggle = screen.getByTestId('route-sheet-snap-toggle');
    const sheetContent = screen.getByTestId('route-sheet-content');

    // Starts inside the base half configuration layout state (50vh)
    expect(sheetContent.className).toContain('h-[50vh]');

    // Fire toggle action to expand to immersive layout constraints (94vh)
    fireEvent.click(snapToggle);
    expect(sheetContent.className).toContain('h-[94vh]');
  });
});

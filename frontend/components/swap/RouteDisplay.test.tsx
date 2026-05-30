import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import fc from 'fast-check';
import { RouteDisplay } from './RouteDisplay';

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

function setReducedMotion(value: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: value,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(() => false),
    }),
  });
}

const DEFAULT_PROPS = {
  amountOut: '10.0000',
  confidenceScore: 85,
  volatility: 'low' as const,
};

// ---------------------------------------------------------------------------
// Unit tests — reduced motion active
// ---------------------------------------------------------------------------

describe('RouteDisplay — reduced motion active', () => {
  afterEach(() => {
    cleanup();
    setReducedMotion(false);
  });

  it('omits transition-all duration-200 from panel container', () => {
    setReducedMotion(true);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const panel = screen.getByTestId('route-display');
    expect(panel.className).not.toContain('transition-all');
    expect(panel.className).not.toContain('duration-200');
  });

  it('omits transition-all duration-150 active:scale-95 from chevron button', () => {
    setReducedMotion(true);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const chevronBtn = screen.getByRole('button', { name: /show route details/i });
    expect(chevronBtn.className).not.toContain('transition-all');
    expect(chevronBtn.className).not.toContain('active:scale-95');
  });

  it('omits transition-transform duration-200 from ChevronDown icon', () => {
    setReducedMotion(true);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const chevronBtn = screen.getByRole('button', { name: /show route details/i });
    const icon = chevronBtn.querySelector('[data-testid="icon"]');
    expect(icon).toBeTruthy();
    expect(icon!.getAttribute('class')).not.toContain('transition-transform');
    expect(icon!.getAttribute('class')).not.toContain('duration-200');
  });

  it('omits transition-colors duration-150 from route summary row', () => {
    setReducedMotion(true);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const panel = screen.getByTestId('route-display');
    // The route summary row is the muted/50 bg div
    const rows = panel.querySelectorAll('.bg-muted\\/50');
    const routeRow = Array.from(rows).find((el) =>
      el.className.includes('rounded-lg')
    );
    expect(routeRow).toBeTruthy();
    expect(routeRow!.className).not.toContain('transition-colors');
    expect(routeRow!.className).not.toContain('duration-150');
  });

  it('omits animate-in slide-in-from-bottom-2 from extended diagnostics panel', async () => {
    setReducedMotion(true);
    render(
      <RouteDisplay
        {...DEFAULT_PROPS}
        extendedRouteDetails
        alternativeRoutes={[
          { id: 'r0', venue: 'AQUA', expectedAmount: '9.9' },
        ]}
      />
    );
    // Click to show details so the diagnostics panel renders
    const chevronBtn = screen.getByRole('button', { name: /show route details/i });
    await userEvent.click(chevronBtn);

    const diag = screen.getByTestId('extended-diagnostics');
    expect(diag.className).not.toContain('animate-in');
    expect(diag.className).not.toContain('slide-in-from-bottom-2');
  });

  it('omits transition-all duration-150 active:scale-[0.99] from alternative route buttons', () => {
    setReducedMotion(true);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const routeBtns = screen.getAllByTestId(/^alternative-route-/);
    routeBtns.forEach((btn) => {
      expect(btn.className).not.toContain('transition-all');
      expect(btn.className).not.toContain('active:scale-[0.99]');
    });
  });
});

// ---------------------------------------------------------------------------
// Unit tests — motion allowed
// ---------------------------------------------------------------------------

describe('RouteDisplay — motion allowed', () => {
  afterEach(() => {
    cleanup();
    setReducedMotion(false);
  });

  it('includes transition-all duration-200 on panel container', () => {
    setReducedMotion(false);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const panel = screen.getByTestId('route-display');
    expect(panel.className).toContain('transition-all');
    expect(panel.className).toContain('duration-200');
  });

  it('includes transition-all duration-150 active:scale-95 on chevron button', () => {
    setReducedMotion(false);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const chevronBtn = screen.getByRole('button', { name: /show route details/i });
    expect(chevronBtn.className).toContain('transition-all');
    expect(chevronBtn.className).toContain('active:scale-95');
  });

  it('includes transition-transform duration-200 on ChevronDown icon', () => {
    setReducedMotion(false);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const chevronBtn = screen.getByRole('button', { name: /show route details/i });
    const icon = chevronBtn.querySelector('[data-testid="icon"]');
    expect(icon!.getAttribute('class')).toContain('transition-transform');
    expect(icon!.getAttribute('class')).toContain('duration-200');
  });

  it('includes animate-in slide-in-from-bottom-2 on extended diagnostics panel', async () => {
    setReducedMotion(false);
    render(
      <RouteDisplay
        {...DEFAULT_PROPS}
        extendedRouteDetails
        alternativeRoutes={[
          { id: 'r0', venue: 'AQUA', expectedAmount: '9.9' },
        ]}
      />
    );
    const chevronBtn = screen.getByRole('button', { name: /show route details/i });
    await userEvent.click(chevronBtn);

    const diag = screen.getByTestId('extended-diagnostics');
    expect(diag.className).toContain('animate-in');
    expect(diag.className).toContain('slide-in-from-bottom-2');
  });

  it('includes transition-all duration-150 active:scale-[0.99] on alternative route buttons', () => {
    setReducedMotion(false);
    render(<RouteDisplay {...DEFAULT_PROPS} />);
    const routeBtns = screen.getAllByTestId(/^alternative-route-/);
    routeBtns.forEach((btn) => {
      expect(btn.className).toContain('transition-all');
      expect(btn.className).toContain('active:scale-[0.99]');
    });
  });
});

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

describe('RouteDisplay — alternative routes with delta badges', () => {
  afterEach(() => {
    cleanup();
    setReducedMotion(false);
  });

  it('renders loading state for routes', () => {
    render(
      <RouteDisplay {...DEFAULT_PROPS} isRoutesLoading alternativeRoutes={[]} />
    );
    expect(screen.getByText('Loading routes...')).toBeInTheDocument();
  });

  it('renders error state for routes', () => {
    render(
      <RouteDisplay
        {...DEFAULT_PROPS}
        routesError="Failed to fetch routes"
        alternativeRoutes={[]}
      />
    );
    expect(screen.getByTestId('routes-error')).toBeInTheDocument();
    expect(screen.getByText('Failed to fetch routes')).toBeInTheDocument();
  });

  it('renders empty state when no routes available', () => {
    render(<RouteDisplay {...DEFAULT_PROPS} alternativeRoutes={[]} />);
    expect(screen.getByTestId('routes-empty')).toBeInTheDocument();
    expect(
      screen.getByText('No alternative routes available')
    ).toBeInTheDocument();
  });

  it('renders alternative route buttons with delta badges', () => {
    const routes = [
      { id: 'r1', venue: 'AQUA Pool', expectedAmount: '≈ 10.0000', score: 95, impactBps: 30, hopCount: 1 },
      { id: 'r2', venue: 'SDEX', expectedAmount: '≈ 9.9500', score: 90, impactBps: 40, hopCount: 2 },
    ];
    render(
      <RouteDisplay {...DEFAULT_PROPS} alternativeRoutes={routes} />
    );
    expect(screen.getByTestId('alternative-route-r1')).toBeInTheDocument();
    expect(screen.getByTestId('alternative-route-r2')).toBeInTheDocument();
  });

  it('calls onSelect when an alternative route is clicked', async () => {
    const onSelect = vi.fn();
    const routes = [
      { id: 'r1', venue: 'AQUA Pool', expectedAmount: '≈ 10.0000' },
      { id: 'r2', venue: 'SDEX', expectedAmount: '≈ 9.9500' },
    ];
    const user = userEvent.setup();
    render(
      <RouteDisplay
        {...DEFAULT_PROPS}
        alternativeRoutes={routes}
        onSelect={onSelect}
      />
    );
    await user.click(screen.getByTestId('alternative-route-r2'));
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'r2' })
    );
  });

  it('shows venue badge labels on route buttons', () => {
    const routes = [
      { id: 'r1', venue: 'SDEX', expectedAmount: '≈ 10.0000' },
      { id: 'r2', venue: 'amm:pool-abc123', expectedAmount: '≈ 9.9500' },
      { id: 'r3', venue: 'Blend Pool', expectedAmount: '≈ 9.9000' },
    ];
    render(<RouteDisplay {...DEFAULT_PROPS} alternativeRoutes={routes} />);
    expect(screen.getByText('SDEX')).toBeInTheDocument();
    expect(screen.getByText('AMM')).toBeInTheDocument();
  });
});

describe('RouteDisplay — property tests', () => {
  afterEach(() => {
    cleanup();
    setReducedMotion(false);
  });

  it(
    // Feature: reduced-motion-swap-animations, Property 7 & 8
    'Property 7 & 8: transition classes absent iff prefersReducedMotion is true',
    () => {
      fc.assert(
        fc.property(fc.boolean(), (prefersReduced) => {
          setReducedMotion(prefersReduced);
          const { unmount } = render(<RouteDisplay {...DEFAULT_PROPS} />);

          const panel = screen.getByTestId('route-display');
          const hasTransition = panel.className.includes('transition-all');

          const routeBtns = screen.getAllByTestId(/^alternative-route-/);
          const btnHasTransition = routeBtns.some((btn) =>
            btn.className.includes('transition-all')
          );

          unmount();

          if (prefersReduced) {
            return !hasTransition && !btnHasTransition;
          } else {
            return hasTransition && btnHasTransition;
          }
        }),
        { numRuns: 100 }
      );
    }
  );
});

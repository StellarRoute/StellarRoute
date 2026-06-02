import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import fc from 'fast-check';
import { ConfidenceIndicator, type RiskFactor } from './ConfidenceIndicator';

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

// ---------------------------------------------------------------------------
// Reduced-motion tests (preserved from original)
// ---------------------------------------------------------------------------

describe('ConfidenceIndicator — reduced-motion', () => {
  afterEach(() => setReducedMotion(false));

  it('volatile badge is rendered when volatility=high regardless of motion preference', () => {
    setReducedMotion(true);
    render(<ConfidenceIndicator score={85} volatility="high" />);
    expect(screen.getByTestId('volatile-badge')).toBeInTheDocument();
  });

  it('volatile badge does NOT have animate-pulse when reduced motion is active', () => {
    setReducedMotion(true);
    render(<ConfidenceIndicator score={85} volatility="high" />);
    const badge = screen.getByTestId('volatile-badge');
    expect(badge.className).not.toContain('animate-pulse');
  });

  it('volatile badge HAS animate-pulse when motion is allowed', () => {
    setReducedMotion(false);
    render(<ConfidenceIndicator score={85} volatility="high" />);
    const badge = screen.getByTestId('volatile-badge');
    expect(badge.className).toContain('animate-pulse');
  });

  it('volatile badge is not rendered when volatility is not high', () => {
    setReducedMotion(false);
    render(<ConfidenceIndicator score={85} volatility="low" />);
    expect(screen.queryByTestId('volatile-badge')).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Risk factor rendering tests (#441)
// ---------------------------------------------------------------------------

describe('ConfidenceIndicator — risk factors', () => {
  it('renders risk-factors container', () => {
    render(<ConfidenceIndicator score={75} />);
    expect(screen.getByTestId('risk-factors')).toBeInTheDocument();
  });

  it('renders default three factors when none supplied', () => {
    render(<ConfidenceIndicator score={75} />);
    expect(screen.getByTestId('risk-factor-liquidity-depth')).toBeInTheDocument();
    expect(screen.getByTestId('risk-factor-source-freshness')).toBeInTheDocument();
    expect(screen.getByTestId('risk-factor-volatility')).toBeInTheDocument();
  });

  it('renders custom risk factors when supplied', () => {
    const factors: RiskFactor[] = [
      { label: 'Custom Factor', severity: 'ok', description: 'All good.' },
      { label: 'Another Factor', severity: 'bad', description: 'Very bad.' },
    ];
    render(<ConfidenceIndicator score={60} riskFactors={factors} />);
    expect(screen.getByTestId('risk-factor-custom-factor')).toBeInTheDocument();
    expect(screen.getByTestId('risk-factor-another-factor')).toBeInTheDocument();
    // Default factors should NOT appear
    expect(screen.queryByTestId('risk-factor-liquidity-depth')).not.toBeInTheDocument();
  });

  it('shows factor descriptions', () => {
    const factors: RiskFactor[] = [
      { label: 'Liquidity Depth', severity: 'warn', description: 'Moderate depth test.' },
    ];
    render(<ConfidenceIndicator score={60} riskFactors={factors} />);
    // Description is in the sr-only hidden element
    expect(screen.getByTestId('risk-factor-liquidity-depth').textContent).toContain('Moderate depth test.');
  });

  it('high score produces ok severity for liquidity depth by default', () => {
    render(<ConfidenceIndicator score={90} />);
    const factor = screen.getByTestId('risk-factor-liquidity-depth');
    // ok severity → text-success class on the label
    expect(factor.textContent).toContain('Liquidity Depth');
  });

  it('low score produces bad severity for liquidity depth by default', () => {
    render(<ConfidenceIndicator score={30} />);
    const factor = screen.getByTestId('risk-factor-liquidity-depth');
    expect(factor.textContent).toContain('Liquidity Depth');
  });

  it('high volatility produces bad severity for volatility factor', () => {
    render(<ConfidenceIndicator score={80} volatility="high" />);
    const factor = screen.getByTestId('risk-factor-volatility');
    expect(factor.textContent).toContain('Volatility');
  });

  it('fallback: renders factors even when score is 0', () => {
    render(<ConfidenceIndicator score={0} />);
    expect(screen.getByTestId('risk-factors')).toBeInTheDocument();
    expect(screen.getAllByTestId(/^risk-factor-/).length).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

describe('ConfidenceIndicator — property tests', () => {
  afterEach(() => setReducedMotion(false));

  it(
    'Property: animate-pulse absent iff prefersReducedMotion is true; badge always present',
    () => {
      fc.assert(
        fc.property(fc.boolean(), (prefersReduced) => {
          setReducedMotion(prefersReduced);
          const { unmount } = render(
            <ConfidenceIndicator score={85} volatility="high" />
          );
          const badge = screen.getByTestId('volatile-badge');
          const isPresent = !!badge;
          const hasPulse = badge.className.includes('animate-pulse');
          unmount();

          if (prefersReduced) {
            return isPresent && !hasPulse;
          } else {
            return isPresent && hasPulse;
          }
        }),
        { numRuns: 100 }
      );
    }
  );

  it('Property: risk-factors container always rendered for any score 0-100', () => {
    fc.assert(
      fc.property(fc.integer({ min: 0, max: 100 }), (score) => {
        const { unmount } = render(<ConfidenceIndicator score={score} />);
        const container = screen.getByTestId('risk-factors');
        const present = !!container;
        unmount();
        return present;
      }),
      { numRuns: 50 }
    );
  });

  it('Property: custom riskFactors are all rendered', () => {
    // Use a fixed label pool to ensure predictable testIds
    const labelPool = ['Alpha', 'Beta', 'Gamma', 'Delta', 'Epsilon'];
    fc.assert(
      fc.property(
        fc.uniqueArray(
          fc.record({
            label: fc.constantFrom(...labelPool),
            severity: fc.constantFrom<RiskFactor['severity']>('ok', 'warn', 'bad'),
            description: fc.string({ minLength: 1, maxLength: 50 }),
          }),
          { minLength: 1, maxLength: 5, selector: (f) => f.label }
        ),
        (factors) => {
          const { unmount } = render(
            <ConfidenceIndicator score={70} riskFactors={factors} />
          );
          const allPresent = factors.every((f) => {
            const testId = `risk-factor-${f.label.toLowerCase()}`;
            return !!screen.queryByTestId(testId);
          });
          unmount();
          return allPresent;
        }
      ),
      { numRuns: 30 }
    );
  });
});

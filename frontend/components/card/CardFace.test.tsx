import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CardFace } from './CardFace';

describe('CardFace', () => {
  const mockCardData = {
    last4: '4567',
    expiry: '12/25',
    usdcAvailable: '1,234.56',
  };

  describe('Rendering', () => {
    it('renders card with all essential information', () => {
      render(<CardFace {...mockCardData} />);

      // Check last 4 digits
      expect(screen.getByText('4567')).toBeInTheDocument();

      // Check expiry
      expect(screen.getByText('12/25')).toBeInTheDocument();

      // Check USDC available
      expect(screen.getByText('1,234.56 USDC')).toBeInTheDocument();

      // Check settlement info
      expect(
        screen.getByText(/charges settle in fiat/i)
      ).toBeInTheDocument();
    });

    it('renders StellarRoute branding', () => {
      render(<CardFace {...mockCardData} />);

      expect(screen.getByText('StellarRoute')).toBeInTheDocument();
      expect(screen.getByText('USDC Card')).toBeInTheDocument();
    });

    it('renders masked card number sections', () => {
      render(<CardFace {...mockCardData} />);

      const maskedSections = screen.getAllByText('••••');
      expect(maskedSections).toHaveLength(3);
    });

    it('displays expires label', () => {
      render(<CardFace {...mockCardData} />);

      expect(screen.getByText('Expires')).toBeInTheDocument();
    });

    it('displays available label', () => {
      render(<CardFace {...mockCardData} />);

      expect(screen.getByText('Available')).toBeInTheDocument();
    });
  });

  describe('Frozen State', () => {
    it('shows frozen overlay when isFrozen is true', () => {
      render(<CardFace {...mockCardData} isFrozen={true} />);

      expect(screen.getByText('Spending Paused')).toBeInTheDocument();
      expect(
        screen.getByText('Card is temporarily frozen')
      ).toBeInTheDocument();
    });

    it('does not show frozen overlay when isFrozen is false', () => {
      render(<CardFace {...mockCardData} isFrozen={false} />);

      expect(screen.queryByText('Spending Paused')).not.toBeInTheDocument();
    });

    it('does not show frozen overlay by default', () => {
      render(<CardFace {...mockCardData} />);

      expect(screen.queryByText('Spending Paused')).not.toBeInTheDocument();
    });

    it('applies grayscale and reduced opacity when frozen', () => {
      const { container } = render(
        <CardFace {...mockCardData} isFrozen={true} />
      );

      const cardElement = container.firstChild as HTMLElement;
      expect(cardElement.className).toContain('grayscale');
      expect(cardElement.className).toContain('opacity-60');
    });

    it('does not apply grayscale when not frozen', () => {
      const { container } = render(
        <CardFace {...mockCardData} isFrozen={false} />
      );

      const cardElement = container.firstChild as HTMLElement;
      expect(cardElement.className).not.toContain('grayscale');
      expect(cardElement.className).not.toContain('opacity-60');
    });
  });

  describe('Accessibility', () => {
    it('has appropriate ARIA label when not frozen', () => {
      render(<CardFace {...mockCardData} />);

      const cardElement = screen.getByRole('img');
      expect(cardElement).toHaveAttribute(
        'aria-label',
        'Card ending in 4567, expires 12/25'
      );
    });

    it('has appropriate ARIA label when frozen', () => {
      render(<CardFace {...mockCardData} isFrozen={true} />);

      const cardElement = screen.getByRole('img');
      expect(cardElement).toHaveAttribute(
        'aria-label',
        'Card spending is paused'
      );
    });

    it('renders as img role for screen readers', () => {
      render(<CardFace {...mockCardData} />);

      expect(screen.getByRole('img')).toBeInTheDocument();
    });
  });

  describe('Custom Styling', () => {
    it('accepts and applies custom className', () => {
      const { container } = render(
        <CardFace {...mockCardData} className="custom-class" />
      );

      const cardElement = container.firstChild as HTMLElement;
      expect(cardElement.className).toContain('custom-class');
    });

    it('preserves default classes when custom className is provided', () => {
      const { container } = render(
        <CardFace {...mockCardData} className="custom-class" />
      );

      const cardElement = container.firstChild as HTMLElement;
      expect(cardElement.className).toContain('rounded-2xl');
      expect(cardElement.className).toContain('custom-class');
    });
  });

  describe('Data Formatting', () => {
    it('handles different USDC amount formats', () => {
      render(<CardFace {...mockCardData} usdcAvailable="10.00" />);
      expect(screen.getByText('10.00 USDC')).toBeInTheDocument();
    });

    it('handles large USDC amounts with commas', () => {
      render(<CardFace {...mockCardData} usdcAvailable="1,000,000.00" />);
      expect(screen.getByText('1,000,000.00 USDC')).toBeInTheDocument();
    });

    it('handles zero USDC balance', () => {
      render(<CardFace {...mockCardData} usdcAvailable="0.00" />);
      expect(screen.getByText('0.00 USDC')).toBeInTheDocument();
    });

    it('handles various expiry date formats', () => {
      render(<CardFace {...mockCardData} expiry="01/30" />);
      expect(screen.getByText('01/30')).toBeInTheDocument();
    });

    it('handles different last4 values', () => {
      render(<CardFace {...mockCardData} last4="0000" />);
      expect(screen.getByText('0000')).toBeInTheDocument();
    });
  });

  describe('Layout and Spacing', () => {
    it('renders with spacious layout', () => {
      const { container } = render(<CardFace {...mockCardData} />);

      const cardElement = container.firstChild as HTMLElement;
      // Check for padding
      expect(cardElement.className).toContain('p-6');
      // Check for max width
      expect(cardElement.className).toContain('max-w-md');
    });

    it('maintains card aspect ratio', () => {
      const { container } = render(<CardFace {...mockCardData} />);

      const cardElement = container.firstChild as HTMLElement;
      // Standard credit card aspect ratio (approximately 1.586:1)
      expect(cardElement.className).toContain('aspect-[1.586/1]');
    });
  });

  describe('Edge Cases', () => {
    it('renders with empty string values', () => {
      render(<CardFace last4="" expiry="" usdcAvailable="" />);

      // Component should render without crashing
      expect(screen.getByText('StellarRoute')).toBeInTheDocument();
    });

    it('handles special characters in last4', () => {
      render(<CardFace {...mockCardData} last4="****" />);
      expect(screen.getByText('****')).toBeInTheDocument();
    });

    it('handles long USDC values gracefully', () => {
      const longValue = '999,999,999.99';
      render(<CardFace {...mockCardData} usdcAvailable={longValue} />);
      expect(screen.getByText(`${longValue} USDC`)).toBeInTheDocument();
    });
  });

  describe('Visual States', () => {
    it('applies hover effect when not frozen', () => {
      const { container } = render(
        <CardFace {...mockCardData} isFrozen={false} />
      );

      const cardElement = container.firstChild as HTMLElement;
      expect(cardElement.className).toContain('hover:shadow-2xl');
    });

    it('has gradient background', () => {
      const { container } = render(<CardFace {...mockCardData} />);

      const cardElement = container.firstChild as HTMLElement;
      expect(cardElement.className).toContain('bg-gradient-to-br');
      expect(cardElement.className).toContain('from-slate-800');
      expect(cardElement.className).toContain('to-slate-900');
    });

    it('applies shadow effects', () => {
      const { container } = render(<CardFace {...mockCardData} />);

      const cardElement = container.firstChild as HTMLElement;
      expect(cardElement.className).toContain('shadow-xl');
    });
  });
});

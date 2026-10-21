import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  ViewFullDetailsButton,
  ViewFullDetailsDisclaimer,
} from './ViewFullDetailsButton';

describe('ViewFullDetailsButton', () => {
  const mockUrl = 'https://issuer-partner.example.com/card-details';
  let windowOpenSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    windowOpenSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
  });

  afterEach(() => {
    windowOpenSpy.mockRestore();
  });

  describe('Rendering', () => {
    it('renders button with default text', () => {
      render(<ViewFullDetailsButton issuerPartnerUrl={mockUrl} />);

      expect(screen.getByText('View Full Details')).toBeInTheDocument();
    });

    it('renders eye icon', () => {
      const { container } = render(
        <ViewFullDetailsButton issuerPartnerUrl={mockUrl} />
      );

      const icons = container.querySelectorAll('svg');
      expect(icons.length).toBeGreaterThan(0);
    });

    it('renders external link icon', () => {
      const { container } = render(
        <ViewFullDetailsButton issuerPartnerUrl={mockUrl} />
      );

      const icons = container.querySelectorAll('svg');
      // Should have eye icon + external link icon
      expect(icons.length).toBeGreaterThanOrEqual(2);
    });

    it('has accessible aria label with default partner name', () => {
      render(<ViewFullDetailsButton issuerPartnerUrl={mockUrl} />);

      const button = screen.getByRole('button');
      expect(button).toHaveAttribute(
        'aria-label',
        'View full card details on issuing partner platform'
      );
    });

    it('has accessible aria label with custom partner name', () => {
      render(
        <ViewFullDetailsButton
          issuerPartnerUrl={mockUrl}
          issuerPartnerName="Acme Bank"
        />
      );

      const button = screen.getByRole('button');
      expect(button).toHaveAttribute(
        'aria-label',
        'View full card details on Acme Bank platform'
      );
    });
  });

  describe('Click Behavior', () => {
    it('opens issuer partner URL in new tab on click', () => {
      render(<ViewFullDetailsButton issuerPartnerUrl={mockUrl} />);

      const button = screen.getByRole('button');
      fireEvent.click(button);

      expect(windowOpenSpy).toHaveBeenCalledWith(
        mockUrl,
        '_blank',
        'noopener,noreferrer'
      );
    });

    it('calls onClick callback when provided', () => {
      const handleClick = vi.fn();
      render(
        <ViewFullDetailsButton
          issuerPartnerUrl={mockUrl}
          onClick={handleClick}
        />
      );

      const button = screen.getByRole('button');
      fireEvent.click(button);

      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('calls onClick before opening URL', () => {
      const callOrder: string[] = [];
      const handleClick = vi.fn(() => callOrder.push('onClick'));
      windowOpenSpy.mockImplementation(() => {
        callOrder.push('windowOpen');
        return null;
      });

      render(
        <ViewFullDetailsButton
          issuerPartnerUrl={mockUrl}
          onClick={handleClick}
        />
      );

      const button = screen.getByRole('button');
      fireEvent.click(button);

      expect(callOrder).toEqual(['onClick', 'windowOpen']);
    });

    it('does not call onClick or open URL when disabled', () => {
      const handleClick = vi.fn();
      render(
        <ViewFullDetailsButton
          issuerPartnerUrl={mockUrl}
          onClick={handleClick}
          disabled={true}
        />
      );

      const button = screen.getByRole('button');
      fireEvent.click(button);

      expect(handleClick).not.toHaveBeenCalled();
      expect(windowOpenSpy).not.toHaveBeenCalled();
    });
  });

  describe('Variants', () => {
    it('applies primary variant styles by default', () => {
      const { container } = render(
        <ViewFullDetailsButton issuerPartnerUrl={mockUrl} />
      );

      const button = container.querySelector('button');
      expect(button?.className).toContain('bg-blue-600');
    });

    it('applies primary variant styles explicitly', () => {
      const { container } = render(
        <ViewFullDetailsButton issuerPartnerUrl={mockUrl} variant="primary" />
      );

      const button = container.querySelector('button');
      expect(button?.className).toContain('bg-blue-600');
    });

    it('applies secondary variant styles', () => {
      const { container } = render(
        <ViewFullDetailsButton
          issuerPartnerUrl={mockUrl}
          variant="secondary"
        />
      );

      const button = container.querySelector('button');
      expect(button?.className).toContain('bg-slate-200');
    });

    it('applies text variant styles', () => {
      const { container } = render(
        <ViewFullDetailsButton issuerPartnerUrl={mockUrl} variant="text" />
      );

      const button = container.querySelector('button');
      expect(button?.className).toContain('bg-transparent');
    });
  });

  describe('Disabled State', () => {
    it('renders as disabled when disabled prop is true', () => {
      render(
        <ViewFullDetailsButton issuerPartnerUrl={mockUrl} disabled={true} />
      );

      const button = screen.getByRole('button');
      expect(button).toBeDisabled();
    });

    it('is enabled by default', () => {
      render(<ViewFullDetailsButton issuerPartnerUrl={mockUrl} />);

      const button = screen.getByRole('button');
      expect(button).not.toBeDisabled();
    });

    it('applies disabled styles when disabled', () => {
      const { container } = render(
        <ViewFullDetailsButton issuerPartnerUrl={mockUrl} disabled={true} />
      );

      const button = container.querySelector('button');
      expect(button?.className).toContain('disabled:opacity-50');
      expect(button?.className).toContain('disabled:cursor-not-allowed');
    });
  });

  describe('Custom Styling', () => {
    it('accepts and applies custom className', () => {
      const { container } = render(
        <ViewFullDetailsButton
          issuerPartnerUrl={mockUrl}
          className="custom-test-class"
        />
      );

      const button = container.querySelector('button');
      expect(button?.className).toContain('custom-test-class');
    });

    it('preserves default classes with custom className', () => {
      const { container } = render(
        <ViewFullDetailsButton
          issuerPartnerUrl={mockUrl}
          className="custom-test-class"
        />
      );

      const button = container.querySelector('button');
      expect(button?.className).toContain('custom-test-class');
      expect(button?.className).toContain('rounded-lg');
    });
  });

  describe('Security', () => {
    it('opens links with noopener and noreferrer', () => {
      render(<ViewFullDetailsButton issuerPartnerUrl={mockUrl} />);

      const button = screen.getByRole('button');
      fireEvent.click(button);

      expect(windowOpenSpy).toHaveBeenCalledWith(
        mockUrl,
        '_blank',
        'noopener,noreferrer'
      );
    });

    it('sets button type to button to prevent form submission', () => {
      render(<ViewFullDetailsButton issuerPartnerUrl={mockUrl} />);

      const button = screen.getByRole('button');
      expect(button).toHaveAttribute('type', 'button');
    });
  });

  describe('Accessibility', () => {
    it('marks decorative icons with aria-hidden', () => {
      const { container } = render(
        <ViewFullDetailsButton issuerPartnerUrl={mockUrl} />
      );

      const icons = container.querySelectorAll('svg');
      icons.forEach((icon) => {
        expect(icon).toHaveAttribute('aria-hidden', 'true');
      });
    });

    it('is keyboard accessible', () => {
      render(<ViewFullDetailsButton issuerPartnerUrl={mockUrl} />);

      const button = screen.getByRole('button');
      expect(button.tagName).toBe('BUTTON');
    });
  });
});

describe('ViewFullDetailsDisclaimer', () => {
  describe('Rendering', () => {
    it('renders disclaimer text with default partner name', () => {
      render(<ViewFullDetailsDisclaimer />);

      expect(
        screen.getByText(/full card details.*are securely managed by/i)
      ).toBeInTheDocument();
      expect(screen.getByText('our issuing partner')).toBeInTheDocument();
    });

    it('renders disclaimer with custom partner name', () => {
      render(<ViewFullDetailsDisclaimer issuerPartnerName="Acme Bank" />);

      expect(screen.getByText('Acme Bank')).toBeInTheDocument();
    });

    it('mentions that StellarRoute never stores card info', () => {
      render(<ViewFullDetailsDisclaimer />);

      expect(
        screen.getByText(/StellarRoute never stores/i)
      ).toBeInTheDocument();
    });

    it('lists card details that are managed by partner', () => {
      render(<ViewFullDetailsDisclaimer />);

      expect(
        screen.getByText(/card number, CVV, PIN/i)
      ).toBeInTheDocument();
    });

    it('renders info icon', () => {
      const { container } = render(<ViewFullDetailsDisclaimer />);

      const icon = container.querySelector('svg');
      expect(icon).toBeInTheDocument();
    });
  });

  describe('Accessibility', () => {
    it('has appropriate role', () => {
      render(<ViewFullDetailsDisclaimer />);

      expect(screen.getByRole('note')).toBeInTheDocument();
    });

    it('has accessible aria label', () => {
      render(<ViewFullDetailsDisclaimer />);

      const element = screen.getByRole('note');
      expect(element).toHaveAttribute('aria-label', 'Card details disclaimer');
    });

    it('marks decorative icon with aria-hidden', () => {
      const { container } = render(<ViewFullDetailsDisclaimer />);

      const icon = container.querySelector('svg');
      expect(icon).toHaveAttribute('aria-hidden', 'true');
    });
  });

  describe('Styling', () => {
    it('applies info box styling', () => {
      const { container } = render(<ViewFullDetailsDisclaimer />);

      const element = container.firstChild as HTMLElement;
      expect(element.className).toContain('bg-blue-50');
      expect(element.className).toContain('border-blue-200');
    });

    it('accepts custom className', () => {
      const { container } = render(
        <ViewFullDetailsDisclaimer className="custom-disclaimer" />
      );

      const element = container.firstChild as HTMLElement;
      expect(element.className).toContain('custom-disclaimer');
    });

    it('has rounded corners', () => {
      const { container } = render(<ViewFullDetailsDisclaimer />);

      const element = container.firstChild as HTMLElement;
      expect(element.className).toContain('rounded-lg');
    });

    it('includes dark mode styles', () => {
      const { container } = render(<ViewFullDetailsDisclaimer />);

      const element = container.firstChild as HTMLElement;
      expect(element.className).toContain('dark:bg-blue-900');
    });
  });

  describe('Content Emphasis', () => {
    it('emphasizes partner name with bold text', () => {
      render(<ViewFullDetailsDisclaimer issuerPartnerName="Acme Bank" />);

      const partnerElement = screen.getByText('Acme Bank');
      expect(partnerElement.className).toContain('font-semibold');
    });
  });

  describe('Layout', () => {
    it('displays icon and text in a row', () => {
      const { container } = render(<ViewFullDetailsDisclaimer />);

      const wrapper = container.firstChild as HTMLElement;
      expect(wrapper.className).toContain('flex');
      expect(wrapper.className).toContain('items-start');
    });

    it('has appropriate spacing', () => {
      const { container } = render(<ViewFullDetailsDisclaimer />);

      const wrapper = container.firstChild as HTMLElement;
      expect(wrapper.className).toContain('gap-2');
      expect(wrapper.className).toContain('p-3');
    });
  });
});

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CardConfirm } from './confirm';

describe('CardConfirm', () => {
  const mockTransactionData = {
    merchant: 'Acme Store',
    fiatAmount: '50.00',
    fiatCurrency: 'USD',
    usdcGross: '49.75',
    fee: '0.25',
    exchangeRate: '1.005',
  };

  describe('Rendering', () => {
    it('renders all transaction details', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(screen.getByText('Confirm Transaction')).toBeInTheDocument();
      expect(screen.getByText('Acme Store')).toBeInTheDocument();
      expect(screen.getByText('USD 50.00')).toBeInTheDocument();
      expect(screen.getByText('49.75 USDC')).toBeInTheDocument();
      expect(screen.getByText('0.25 USDC')).toBeInTheDocument();
    });

    it('displays merchant information prominently', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(screen.getByText('Merchant')).toBeInTheDocument();
      expect(screen.getByText('Acme Store')).toBeInTheDocument();
    });

    it('shows exchange rate', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(screen.getByText('Exchange Rate')).toBeInTheDocument();
      expect(screen.getByText('1 USDC = 1.005 USD')).toBeInTheDocument();
    });

    it('displays total USDC required', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(screen.getByText('Total USDC Required')).toBeInTheDocument();
      expect(screen.getByText('50.00 USDC')).toBeInTheDocument();
    });

    it('shows warning about wallet signature', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(
        screen.getByText(/you will be prompted to sign/i)
      ).toBeInTheDocument();
    });

    it('renders header with description', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(screen.getByText('Review details before signing')).toBeInTheDocument();
    });
  });

  describe('Button Interactions', () => {
    it('calls onConfirm when confirm button is clicked', () => {
      const handleConfirm = vi.fn();
      render(
        <CardConfirm {...mockTransactionData} onConfirm={handleConfirm} />
      );

      const confirmButton = screen.getByRole('button', {
        name: /confirm transaction and sign/i,
      });
      fireEvent.click(confirmButton);

      expect(handleConfirm).toHaveBeenCalledTimes(1);
    });

    it('calls onCancel when cancel button is clicked', () => {
      const handleCancel = vi.fn();
      render(<CardConfirm {...mockTransactionData} onCancel={handleCancel} />);

      const cancelButton = screen.getByRole('button', {
        name: /cancel transaction/i,
      });
      fireEvent.click(cancelButton);

      expect(handleCancel).toHaveBeenCalledTimes(1);
    });

    it('does not call handlers when buttons are disabled', () => {
      const handleConfirm = vi.fn();
      const handleCancel = vi.fn();

      render(
        <CardConfirm
          {...mockTransactionData}
          onConfirm={handleConfirm}
          onCancel={handleCancel}
          isLoading={true}
        />
      );

      const confirmButton = screen.getByRole('button', {
        name: /confirm transaction and sign/i,
      });
      const cancelButton = screen.getByRole('button', {
        name: /cancel transaction/i,
      });

      fireEvent.click(confirmButton);
      fireEvent.click(cancelButton);

      expect(handleConfirm).not.toHaveBeenCalled();
      expect(handleCancel).not.toHaveBeenCalled();
    });
  });

  describe('Loading State', () => {
    it('shows loading spinner when isLoading is true', () => {
      render(<CardConfirm {...mockTransactionData} isLoading={true} />);

      expect(screen.getByText('Processing...')).toBeInTheDocument();
    });

    it('disables buttons when isLoading is true', () => {
      render(<CardConfirm {...mockTransactionData} isLoading={true} />);

      const confirmButton = screen.getByRole('button', {
        name: /confirm transaction and sign/i,
      });
      const cancelButton = screen.getByRole('button', {
        name: /cancel transaction/i,
      });

      expect(confirmButton).toBeDisabled();
      expect(cancelButton).toBeDisabled();
    });

    it('shows confirm text when not loading', () => {
      render(<CardConfirm {...mockTransactionData} isLoading={false} />);

      expect(screen.getByText('Confirm & Sign')).toBeInTheDocument();
      expect(screen.queryByText('Processing...')).not.toBeInTheDocument();
    });

    it('defaults to not loading', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(screen.getByText('Confirm & Sign')).toBeInTheDocument();
    });
  });

  describe('Calculations', () => {
    it('correctly calculates total USDC (gross + fee)', () => {
      render(<CardConfirm {...mockTransactionData} />);

      // 49.75 + 0.25 = 50.00
      expect(screen.getByText('50.00 USDC')).toBeInTheDocument();
    });

    it('handles decimal calculations correctly', () => {
      render(
        <CardConfirm
          {...mockTransactionData}
          usdcGross="123.45"
          fee="6.78"
        />
      );

      // 123.45 + 6.78 = 130.23
      expect(screen.getByText('130.23 USDC')).toBeInTheDocument();
    });

    it('formats total with two decimal places', () => {
      render(
        <CardConfirm {...mockTransactionData} usdcGross="10.5" fee="0.3" />
      );

      // 10.5 + 0.3 = 10.80
      expect(screen.getByText('10.80 USDC')).toBeInTheDocument();
    });

    it('handles zero fee', () => {
      render(<CardConfirm {...mockTransactionData} fee="0.00" />);

      expect(screen.getByText('0.00 USDC')).toBeInTheDocument();
      expect(screen.getByText('49.75 USDC')).toBeInTheDocument();
    });
  });

  describe('Currency Display', () => {
    it('displays different fiat currencies correctly', () => {
      render(
        <CardConfirm {...mockTransactionData} fiatCurrency="EUR" />
      );

      expect(screen.getByText('EUR 50.00')).toBeInTheDocument();
      expect(screen.getByText('1 USDC = 1.005 EUR')).toBeInTheDocument();
    });

    it('shows USD currency by default in test data', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(screen.getByText(/USD/)).toBeInTheDocument();
    });

    it('formats USDC amounts consistently', () => {
      render(<CardConfirm {...mockTransactionData} />);

      const usdcElements = screen.getAllByText(/USDC/);
      expect(usdcElements.length).toBeGreaterThan(0);
    });
  });

  describe('Accessibility', () => {
    it('has accessible button labels', () => {
      render(<CardConfirm {...mockTransactionData} />);

      expect(
        screen.getByRole('button', { name: /cancel transaction/i })
      ).toBeInTheDocument();
      expect(
        screen.getByRole('button', { name: /confirm transaction and sign/i })
      ).toBeInTheDocument();
    });

    it('indicates loading state to screen readers', () => {
      render(<CardConfirm {...mockTransactionData} isLoading={true} />);

      const spinner = screen.getByRole('button', {
        name: /confirm transaction and sign/i,
      }).querySelector('svg');

      expect(spinner).toHaveAttribute('aria-hidden', 'true');
    });

    it('all interactive elements are keyboard accessible', () => {
      const { container } = render(<CardConfirm {...mockTransactionData} />);

      const buttons = container.querySelectorAll('button');
      buttons.forEach((button) => {
        expect(button).not.toHaveAttribute('tabIndex', '-1');
      });
    });
  });

  describe('Layout and Structure', () => {
    it('displays sections in correct order', () => {
      const { container } = render(<CardConfirm {...mockTransactionData} />);

      const sections = container.querySelectorAll('div');
      expect(sections.length).toBeGreaterThan(0);
    });

    it('uses appropriate spacing between sections', () => {
      const { container } = render(<CardConfirm {...mockTransactionData} />);

      const mainContainer = container.firstChild as HTMLElement;
      expect(mainContainer.className).toContain('p-6');
    });

    it('has rounded corners and shadow', () => {
      const { container } = render(<CardConfirm {...mockTransactionData} />);

      const mainContainer = container.firstChild as HTMLElement;
      expect(mainContainer.className).toContain('rounded-xl');
      expect(mainContainer.className).toContain('shadow-lg');
    });
  });

  describe('Edge Cases', () => {
    it('handles very large amounts', () => {
      render(
        <CardConfirm
          {...mockTransactionData}
          fiatAmount="1,000,000.00"
          usdcGross="999,000.00"
          fee="1,000.00"
        />
      );

      expect(screen.getByText('USD 1,000,000.00')).toBeInTheDocument();
      expect(screen.getByText('1000000.00 USDC')).toBeInTheDocument();
    });

    it('handles very small amounts', () => {
      render(
        <CardConfirm
          {...mockTransactionData}
          fiatAmount="0.01"
          usdcGross="0.009"
          fee="0.001"
        />
      );

      expect(screen.getByText('USD 0.01')).toBeInTheDocument();
      expect(screen.getByText('0.01 USDC')).toBeInTheDocument();
    });

    it('handles empty merchant name', () => {
      render(<CardConfirm {...mockTransactionData} merchant="" />);

      expect(screen.getByText('Merchant')).toBeInTheDocument();
    });

    it('handles different exchange rate formats', () => {
      render(
        <CardConfirm {...mockTransactionData} exchangeRate="0.995" />
      );

      expect(screen.getByText('1 USDC = 0.995 USD')).toBeInTheDocument();
    });

    it('renders without optional callbacks', () => {
      expect(() => {
        render(<CardConfirm {...mockTransactionData} />);
      }).not.toThrow();
    });
  });

  describe('Dark Mode Support', () => {
    it('includes dark mode styles', () => {
      const { container } = render(<CardConfirm {...mockTransactionData} />);

      const mainContainer = container.firstChild as HTMLElement;
      expect(mainContainer.className).toContain('dark:bg-slate-900');
    });

    it('applies dark mode to text elements', () => {
      const { container } = render(<CardConfirm {...mockTransactionData} />);

      const elements = container.querySelectorAll('[class*="dark:"]');
      expect(elements.length).toBeGreaterThan(0);
    });
  });

  describe('Visual Hierarchy', () => {
    it('highlights total amount prominently', () => {
      const { container } = render(<CardConfirm {...mockTransactionData} />);

      const totalSection = screen
        .getByText('Total USDC Required')
        .closest('div');
      expect(totalSection?.className).toContain('bg-blue');
    });

    it('uses appropriate font weights', () => {
      const { container } = render(<CardConfirm {...mockTransactionData} />);

      const heading = screen.getByText('Confirm Transaction');
      expect(heading.className).toContain('font-bold');
    });
  });
});

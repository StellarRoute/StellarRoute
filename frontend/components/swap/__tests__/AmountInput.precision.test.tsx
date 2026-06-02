import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { AmountInput } from '../AmountInput';

describe('AmountInput — adaptive precision (#442)', () => {
  // -----------------------------------------------------------------------
  // resolveMaxDecimals via assetId
  // -----------------------------------------------------------------------

  it('defaults to 7 decimals for native asset', () => {
    const onChange = vi.fn();
    render(<AmountInput value="" assetId="native" onChange={onChange} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    // 7 decimal places should be accepted without error
    fireEvent.change(input, { target: { value: '1.1234567' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('clamps input to 7 decimals for native and shows error', () => {
    const onChange = vi.fn();
    render(<AmountInput value="" assetId="native" onChange={onChange} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '1.12345678' } }); // 8 decimals
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByRole('alert').textContent).toMatch(/7/);
  });

  it('respects explicit decimals=2 over assetId heuristic', () => {
    const onChange = vi.fn();
    render(<AmountInput value="" assetId="native" decimals={2} onChange={onChange} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '1.123' } }); // 3 decimals, exceeds 2
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByRole('alert').textContent).toMatch(/2/);
  });

  it('accepts exactly max decimals without error', () => {
    render(<AmountInput value="" decimals={4} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '0.1234' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  // -----------------------------------------------------------------------
  // Tiny values
  // -----------------------------------------------------------------------

  it('accepts tiny value 0.0000001 within 7 decimals', () => {
    render(<AmountInput value="" assetId="native" onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '0.0000001' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('clamps tiny value with too many decimals', () => {
    render(<AmountInput value="" decimals={2} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '0.001' } }); // 3 decimals, exceeds 2
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  // -----------------------------------------------------------------------
  // Very large values
  // -----------------------------------------------------------------------

  it('accepts very large integer value', () => {
    render(<AmountInput value="" decimals={7} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '999999999' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('accepts very large value with max decimals', () => {
    render(<AmountInput value="" decimals={7} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '999999999.1234567' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  // -----------------------------------------------------------------------
  // decimals=0 (integer-only assets)
  // -----------------------------------------------------------------------

  it('rejects any fractional input when decimals=0', () => {
    render(<AmountInput value="" decimals={0} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '5.1' } });
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('accepts integer input when decimals=0', () => {
    render(<AmountInput value="" decimals={0} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '42' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  // -----------------------------------------------------------------------
  // aria attributes
  // -----------------------------------------------------------------------

  it('sets aria-invalid when precision error is present', () => {
    render(<AmountInput value="" decimals={2} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '1.999' } });
    expect(input).toHaveAttribute('aria-invalid', 'true');
  });

  it('sets aria-describedby pointing to error element', () => {
    render(<AmountInput value="" decimals={2} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '1.999' } });
    const errorId = input.getAttribute('aria-describedby');
    expect(errorId).toBeTruthy();
    expect(document.getElementById(errorId!)).toBeInTheDocument();
  });

  it('clears error when input is corrected to valid value', () => {
    render(<AmountInput value="" decimals={2} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    // Trigger error with 3 decimals
    fireEvent.change(input, { target: { value: '1.999' } });
    expect(screen.getByRole('alert')).toBeInTheDocument();
    // Type a completely different valid value (not the clamped result)
    fireEvent.change(input, { target: { value: '2.5' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('clears error on empty input', () => {
    render(<AmountInput value="" decimals={2} onChange={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '1.999' } });
    fireEvent.change(input, { target: { value: '' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});

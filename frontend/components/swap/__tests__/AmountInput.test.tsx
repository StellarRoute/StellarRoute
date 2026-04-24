import { describe, it, expect } from 'vitest';
import { renderWithHarness, screen } from './harness';
import { AmountInput } from '../AmountInput';

describe('AmountInput Harness Test', () => {
  it('should render with a default value', () => {
    renderWithHarness(<AmountInput value="10" onChange={() => {}} />);
    
    const input = screen.getByRole('textbox') as HTMLInputElement;
    expect(input.value).toBe('10');
  });

  it('should show the balance from the harness data', () => {
    renderWithHarness(<AmountInput value="5" balance="100" onChange={() => {}} />);
    expect(screen.getByText(/100/)).toBeInTheDocument();
  });
});
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CardFreezeButton } from './CardFreezeButton';

function mockApi(overrides: Record<string, unknown> = {}) {
  return {
    lifecycle: 'idle',
    isFrozen: false,
    isBusy: false,
    buttonLabel: 'Freeze Card',
    lastTxHash: null,
    error: null,
    freeze: vi.fn(),
    unfreeze: vi.fn(),
    setIsFrozen: vi.fn(),
    ...overrides,
  } as any;
}

describe('CardFreezeButton', () => {
  it('shows Freeze Card when not frozen', () => {
    render(<CardFreezeButton api={mockApi()} />);
    expect(screen.getByTestId('card-freeze-button').textContent).toBe(
      'Freeze Card',
    );
  });

  it('calls freeze on click', () => {
    const api = mockApi();
    render(<CardFreezeButton api={api} />);
    fireEvent.click(screen.getByTestId('card-freeze-button'));
    expect(api.freeze).toHaveBeenCalled();
  });

  it('shows Awaiting Signature and is disabled when busy', () => {
    render(
      <CardFreezeButton
        api={mockApi({
          lifecycle: 'awaiting_signature',
          isBusy: true,
          buttonLabel: 'Awaiting Signature...',
        })}
      />,
    );
    const btn = screen.getByTestId('card-freeze-button') as HTMLButtonElement;
    expect(btn.textContent).toBe('Awaiting Signature...');
    expect(btn.disabled).toBe(true);
  });

  it('calls unfreeze when frozen', () => {
    const api = mockApi({
      isFrozen: true,
      buttonLabel: 'Unfreeze Card',
    });
    render(<CardFreezeButton api={api} />);
    fireEvent.click(screen.getByTestId('card-freeze-button'));
    expect(api.unfreeze).toHaveBeenCalled();
  });
});
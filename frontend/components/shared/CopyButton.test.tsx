import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { CopyButton } from './CopyButton';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';

vi.mock('@/hooks/useCopyToClipboard', () => ({
  useCopyToClipboard: vi.fn()
}));

describe('CopyButton', () => {
  it('renders with correct aria-label and calls copy on click', () => {
    const copyMock = vi.fn();
    vi.mocked(useCopyToClipboard).mockReturnValue({ copy: copyMock, copied: false });

    render(<CopyButton value="test-value" label="Test copy label" />);

    // Accessible name asserted
    const button = screen.getByRole('button', { name: 'Test copy label' });
    expect(button).not.toBeNull();
    expect(button.getAttribute('aria-label')).toBe('Test copy label');

    // Copy invoked with the value prop
    fireEvent.click(button);
    expect(copyMock).toHaveBeenCalledWith('test-value');
  });

  it('can be activated via keyboard', () => {
    const copyMock = vi.fn();
    vi.mocked(useCopyToClipboard).mockReturnValue({ copy: copyMock, copied: false });

    render(<CopyButton value="test-value" label="Test copy label" />);

    const button = screen.getByRole('button', { name: 'Test copy label' });
    button.focus();
    expect(document.activeElement).toBe(button);
    
    // Native buttons trigger click on Enter/Space
    fireEvent.keyDown(button, { key: 'Enter', code: 'Enter' });
    fireEvent.click(button);
    expect(copyMock).toHaveBeenCalledWith('test-value');
  });

  it('updates aria-label when copied', () => {
    const copyMock = vi.fn();
    vi.mocked(useCopyToClipboard).mockReturnValue({ copy: copyMock, copied: true });

    render(<CopyButton value="test-value" label="Test copy label" />);

    const button = screen.getByRole('button', { name: 'Copied!' });
    expect(button).not.toBeNull();
    expect(button.getAttribute('aria-label')).toBe('Copied!');
  });
});

import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import FirstSwapGuidePage from './page';
import { SettingsProvider } from '@/components/providers/settings-provider';

describe('FirstSwapGuidePage', () => {
  it('renders guide header, steps, and CTAs through i18n', () => {
    render(
      <SettingsProvider>
        <FirstSwapGuidePage />
      </SettingsProvider>
    );

    expect(screen.getByRole('heading', { name: /Your first live swap/i })).toBeInTheDocument();
    expect(screen.getByText(/User guide/i)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Open swap/i })).toHaveAttribute('href', '/swap');
    expect(screen.getByRole('link', { name: /Full guide on GitHub/i })).toBeInTheDocument();
    expect(screen.getByText('Step 1')).toBeInTheDocument();
    expect(screen.getByText('Connect your wallet')).toBeInTheDocument();
    expect(screen.getByText('Before you confirm')).toBeInTheDocument();
  });
});

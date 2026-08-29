import { describe, expect, it } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { SettingsProvider } from '@/components/providers/settings-provider';
import { LocaleSelector } from '@/components/settings/LocaleSelector';
import { SUPPORTED_LOCALES } from '@/lib/formatting';

const STORAGE_KEY = 'stellar_route_settings';

function Wrapper({ children }: { children: React.ReactNode }) {
  return <SettingsProvider>{children}</SettingsProvider>;
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

describe('LocaleSelector', () => {
  it('renders the card title and description', () => {
    render(
      <Wrapper>
        <LocaleSelector />
      </Wrapper>
    );

    expect(screen.getByText('Language & Region')).toBeInTheDocument();
    expect(
      screen.getByText(/choose your preferred language and number formatting/i)
    ).toBeInTheDocument();
  });

  it('renders a button for every supported locale', () => {
    render(
      <Wrapper>
        <LocaleSelector />
      </Wrapper>
    );

    Object.values(SUPPORTED_LOCALES).forEach((displayName) => {
      expect(
        screen.getByRole('button', {
          name: new RegExp(escapeRegExp(displayName)),
        })
      ).toBeInTheDocument();
    });
  });

  it('renders the formatted example for the active locale', () => {
    render(
      <Wrapper>
        <LocaleSelector />
      </Wrapper>
    );

    const english = screen.getByRole('button', {
      name: /English \(United States\)/i,
    });
    expect(
      within(english).getByText('Example: 1,234.56 · 1.23%')
    ).toBeInTheDocument();
  });

  it('renders locale-specific examples using Intl number formatting', () => {
    render(
      <Wrapper>
        <LocaleSelector />
      </Wrapper>
    );

    const german = screen.getByRole('button', {
      name: /Deutsch \(Deutschland\)/i,
    });
    expect(
      within(german).getByText(/Example:\s*1\.234,56/)
    ).toBeInTheDocument();
  });

  it('marks the active locale button as the default variant', () => {
    render(
      <Wrapper>
        <LocaleSelector />
      </Wrapper>
    );

    expect(
      screen.getByRole('button', { name: /English \(United States\)/i })
    ).toHaveAttribute('data-variant', 'default');

    expect(
      screen.getByRole('button', { name: /Deutsch \(Deutschland\)/i })
    ).toHaveAttribute('data-variant', 'outline');
  });

  it('switches the active locale when another option is clicked', async () => {
    const user = userEvent.setup();
    render(
      <Wrapper>
        <LocaleSelector />
      </Wrapper>
    );

    await user.click(
      screen.getByRole('button', { name: /Deutsch \(Deutschland\)/i })
    );

    expect(
      screen.getByRole('button', { name: /Deutsch \(Deutschland\)/i })
    ).toHaveAttribute('data-variant', 'default');

    expect(
      screen.getByRole('button', { name: /English \(United States\)/i })
    ).toHaveAttribute('data-variant', 'outline');

    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? '{}');
    expect(stored.locale).toBe('de-DE');
  });

  it('honours a persisted locale on mount', () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ locale: 'ja-JP' })
    );

    render(
      <Wrapper>
        <LocaleSelector />
      </Wrapper>
    );

    expect(screen.getByRole('button', { name: /日本語/i })).toHaveAttribute(
      'data-variant',
      'default'
    );
  });
});

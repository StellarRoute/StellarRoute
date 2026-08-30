import React from 'react';
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import { hydrateRoot } from 'react-dom/client';
import { act } from 'react';
import { renderHook, waitFor } from '@testing-library/react';
import {
  invalidateFlagCache,
  resolveFlag,
  resolveFlagForInitialRender,
  useFeatureFlag,
  useFeatureFlags,
  type FlagName,
} from './useFeatureFlag';

function mockFetch(flags: Record<string, boolean>) {
  global.fetch = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => flags,
  } as Response);
}

function FlagProbe({ flag }: { flag: FlagName }) {
  const { enabled, loading } = useFeatureFlag(flag);
  if (loading) return <div data-testid="flag-loading">loading</div>;
  return enabled ? (
    <div data-testid="flag-on">on</div>
  ) : (
    <div data-testid="flag-off">off</div>
  );
}

function BatchProbe({ flags }: { flags: FlagName[] }) {
  const resolved = useFeatureFlags(flags);
  return (
    <div data-testid="batch-swap-ui-v2">
      {resolved.swap_ui_v2 ? 'v2' : 'legacy'}
    </div>
  );
}

beforeEach(() => {
  invalidateFlagCache();
  delete process.env.NEXT_PUBLIC_FLAGS_URL;
  delete process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2;
  delete process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA;
  delete (window as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> })
    .__STELLAR_ROUTE_FLAGS__;
});

afterEach(() => {
  delete (window as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> })
    .__STELLAR_ROUTE_FLAGS__;
});

describe('resolveFlagForInitialRender', () => {
  it('uses the default-on hydration snapshot before a window false override', () => {
    (window as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> }).__STELLAR_ROUTE_FLAGS__ = {
      swap_ui_v2: false,
    };
    expect(resolveFlagForInitialRender('swap_ui_v2')).toBe(true);
    expect(resolveFlag('swap_ui_v2')).toBe(false);
  });

  it('reads env for initial snapshot', () => {
    process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2 = 'true';
    expect(resolveFlagForInitialRender('swap_ui_v2')).toBe(true);
  });
});

describe('useFeatureFlag hydration', () => {
  it('window-only false: initial on, no recoverable errors, then off after mount', async () => {
    const serverHtml = renderToString(<FlagProbe flag="swap_ui_v2" />);
    expect(serverHtml).toContain('on');

    (window as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> }).__STELLAR_ROUTE_FLAGS__ = {
      swap_ui_v2: false,
    };

    const container = document.createElement('div');
    container.innerHTML = serverHtml;
    const recoverableErrors: string[] = [];
    const consoleErrors: string[] = [];
    const originalError = console.error;
    console.error = (...args: unknown[]) => {
      const message = String(args[0] ?? '');
      if (message.includes('Hydration')) consoleErrors.push(message);
      originalError(...args);
    };

    act(() => {
      hydrateRoot(container, <FlagProbe flag="swap_ui_v2" />, {
        onRecoverableError: (error) => {
          recoverableErrors.push(
            error instanceof Error ? error.message : String(error),
          );
        },
      });
    });

    console.error = originalError;
    expect(recoverableErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);

    await act(async () => {
      await Promise.resolve();
    });

    expect(container.textContent).toContain('off');
  });

  it('env=false disables swap_ui_v2 for initial and hydrated renders', async () => {
    process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2 = 'false';
    const serverHtml = renderToString(<FlagProbe flag="swap_ui_v2" />);
    expect(serverHtml).toContain('off');

    const container = document.createElement('div');
    container.innerHTML = serverHtml;
    const recoverableErrors: string[] = [];

    act(() => {
      hydrateRoot(container, <FlagProbe flag="swap_ui_v2" />, {
        onRecoverableError: (error) => {
          recoverableErrors.push(
            error instanceof Error ? error.message : String(error),
          );
        },
      });
    });

    expect(recoverableErrors).toEqual([]);

    await act(async () => {
      await Promise.resolve();
    });

    expect(container.textContent).toContain('off');
  });

  it('FLAGS_URL-only path stays loading until remote resolves', async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = 'https://flags.example.com/flags.json';
    mockFetch({ swap_ui_v2: false });

    const { result } = renderHook(() => useFeatureFlag('swap_ui_v2'));
    expect(result.current.loading).toBe(true);
    expect(result.current.enabled).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.enabled).toBe(false);
  });
});

describe('useFeatureFlags hydration', () => {
  it('matches default-on SSR snapshot and applies window false after mount', async () => {
    const serverHtml = renderToString(<BatchProbe flags={['swap_ui_v2']} />);
    expect(serverHtml).toContain('v2');

    (window as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> }).__STELLAR_ROUTE_FLAGS__ = {
      swap_ui_v2: false,
    };

    const container = document.createElement('div');
    container.innerHTML = serverHtml;
    const recoverableErrors: string[] = [];

    act(() => {
      hydrateRoot(container, <BatchProbe flags={['swap_ui_v2']} />, {
        onRecoverableError: (error) => {
          recoverableErrors.push(
            error instanceof Error ? error.message : String(error),
          );
        },
      });
    });

    expect(recoverableErrors).toEqual([]);

    await act(async () => {
      await Promise.resolve();
    });

    expect(container.textContent).toContain('legacy');
  });
});

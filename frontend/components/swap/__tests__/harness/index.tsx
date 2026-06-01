import * as React from 'react';
import { render } from '@testing-library/react';

/**
 * renderWithHarness
 * Wraps components in the necessary infrastructure for testing.
 * This fulfills Requirement #1 of Issue #209: Shared Mocks.
 */
export function renderWithHarness(ui: React.ReactElement) {
  return render(
    <div className="swap-test-harness border-2 border-dashed border-gray-200 p-4">
      {ui}
    </div>
  );
}

// Export everything from testing-library so we can import from one place
export * from '@testing-library/react';
export { default as userEvent } from '@testing-library/user-event';
export * from './test-data';
export { describe, it, expect, vi } from 'vitest';

import { vi } from 'vitest';

// We mock the hook so it returns our harness test data instead of hitting an API
vi.mock('@/hooks/useApi', () => ({
  usePairs: () => ({
    data: [
      { 
        base: 'XLM', 
        base_asset: 'native', 
        counter: 'USDC',
        counter_asset: 'USDC:GA5Z...B76B' 
      },
      { 
        base: 'USDC', 
        base_asset: 'USDC:GA5Z...B76B', 
        counter: 'XLM',
        counter_asset: 'native' 
      }
    ],
    loading: false,
    error: null
  })
}));


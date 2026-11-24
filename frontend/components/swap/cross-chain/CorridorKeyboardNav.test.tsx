import React from 'react';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CorridorTabs } from './CorridorTabs';
import type { CorridorId, CorridorSelectionId } from '@/lib/cross-chain/types';

// ── Feature flag ──────────────────────────────────────────────────────────────
const mockUseFeatureFlag = vi.fn((flag: string) => ({
  enabled: flag === 'corridor_keyboard_nav',
  loading: false,
}));
vi.mock('@/hooks/useFeatureFlag', () => ({
  useFeatureFlag: (flag: string) => mockUseFeatureFlag(flag),
}));

// ── Corridor catalog: two executable corridors ────────────────────────────────
vi.mock('@/hooks/useCorridorCatalog', () => ({
  useCorridorCatalog: () => ({
    corridors: [
      { id: 'evm-to-stellar', label: 'ETH → XLM', executable: true, availability: 'live' },
      { id: 'stellar-to-evm', label: 'XLM → ETH', executable: true, availability: 'live' },
    ],
  }),
}));

vi.mock('@/lib/cross-chain/corridors', () => ({
  UNMATCHED_CORRIDOR_ID: '__unmatched__',
}));

function setup(activeId: CorridorSelectionId = 'evm-to-stellar') {
  const onSelect = vi.fn();
  const user = userEvent.setup();
  render(<CorridorTabs activeId={activeId} onSelect={onSelect} />);
  return { onSelect, user };
}

describe('CorridorTabs keyboard navigation (flag on)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseFeatureFlag.mockImplementation((flag: string) => ({
      enabled: flag === 'corridor_keyboard_nav',
      loading: false,
    }));
  });
  afterEach(cleanup);

  it('renders both corridor tabs', () => {
    setup();
    expect(screen.getByTestId('corridor-tab-evm-to-stellar')).toBeInTheDocument();
    expect(screen.getByTestId('corridor-tab-stellar-to-evm')).toBeInTheDocument();
  });

  it('ArrowRight moves focus to the next tab', async () => {
    const { user } = setup();
    const first = screen.getByTestId('corridor-tab-evm-to-stellar');
    first.focus();
    await user.keyboard('{ArrowRight}');
    expect(document.activeElement).toBe(screen.getByTestId('corridor-tab-stellar-to-evm'));
  });

  it('ArrowLeft moves focus to the previous tab', async () => {
    const { user } = setup();
    const second = screen.getByTestId('corridor-tab-stellar-to-evm');
    second.focus();
    await user.keyboard('{ArrowLeft}');
    expect(document.activeElement).toBe(screen.getByTestId('corridor-tab-evm-to-stellar'));
  });

  it('ArrowRight wraps around from last to first', async () => {
    const { user } = setup();
    const second = screen.getByTestId('corridor-tab-stellar-to-evm');
    second.focus();
    await user.keyboard('{ArrowRight}');
    expect(document.activeElement).toBe(screen.getByTestId('corridor-tab-evm-to-stellar'));
  });

  it('Home focuses the first tab', async () => {
    const { user } = setup();
    const second = screen.getByTestId('corridor-tab-stellar-to-evm');
    second.focus();
    await user.keyboard('{Home}');
    expect(document.activeElement).toBe(screen.getByTestId('corridor-tab-evm-to-stellar'));
  });

  it('End focuses the last tab', async () => {
    const { user } = setup();
    const first = screen.getByTestId('corridor-tab-evm-to-stellar');
    first.focus();
    await user.keyboard('{End}');
    expect(document.activeElement).toBe(screen.getByTestId('corridor-tab-stellar-to-evm'));
  });

  it('calls onSelect with the newly focused corridor id', async () => {
    const { user, onSelect } = setup();
    const first = screen.getByTestId('corridor-tab-evm-to-stellar');
    first.focus();
    await user.keyboard('{ArrowRight}');
    expect(onSelect).toHaveBeenCalledWith('stellar-to-evm');
  });

  it('disabled prop prevents keyboard navigation', async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    render(<CorridorTabs activeId="evm-to-stellar" onSelect={onSelect} disabled />);

    const first = screen.getByTestId('corridor-tab-evm-to-stellar');
    // Tabs are disabled, can't focus them in jsdom
    expect(first).toBeDisabled();
  });
});

describe('CorridorTabs keyboard navigation (flag off)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseFeatureFlag.mockReturnValue({ enabled: false, loading: false });
  });
  afterEach(cleanup);

  it('ArrowRight does not trigger onSelect when flag is off', async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    render(<CorridorTabs activeId="evm-to-stellar" onSelect={onSelect} />);

    const first = screen.getByTestId('corridor-tab-evm-to-stellar');
    first.focus();
    await user.keyboard('{ArrowRight}');
    // onSelect not called by keyboard when flag off
    expect(onSelect).not.toHaveBeenCalled();
  });
});

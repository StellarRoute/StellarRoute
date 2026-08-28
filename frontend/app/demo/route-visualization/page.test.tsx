import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import RouteVisualizationDemo from './page';

describe('RouteVisualizationDemo Page', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders without network or wallet dependencies', () => {
    render(<RouteVisualizationDemo />);

    expect(
      screen.getByRole('heading', { name: 'Route Visualization Demo', level: 1 })
    ).toBeInTheDocument();
    expect(
      screen.getByText('Interactive demo of the multi-hop trade route visualization component')
    ).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Single Hop' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Multi-Hop' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Complex Route' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Split Route' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'States' })).toBeInTheDocument();
  });

  it('renders single hop route tab by default', () => {
    render(<RouteVisualizationDemo />);

    expect(
      screen.getByRole('heading', { name: 'Single Hop Route', level: 2 })
    ).toBeInTheDocument();
    expect(
      screen.getByText('Direct swap from XLM to USDC via SDEX orderbook')
    ).toBeInTheDocument();
  });

  it('allows switching between tabs to inspect different route topologies', async () => {
    const user = userEvent.setup();
    render(<RouteVisualizationDemo />);

    // Multi-Hop tab
    await user.click(screen.getByRole('tab', { name: 'Multi-Hop' }));
    expect(
      await screen.findByRole('heading', { name: 'Multi-Hop Route', level: 2 })
    ).toBeInTheDocument();
    expect(
      screen.getByText('XLM → USDC (SDEX) → BTC (AMM Pool)')
    ).toBeInTheDocument();

    // Complex Route tab
    await user.click(screen.getByRole('tab', { name: 'Complex Route' }));
    expect(
      await screen.findByRole('heading', { name: 'Complex 3-Hop Route', level: 2 })
    ).toBeInTheDocument();
    expect(
      screen.getByText('XLM → USDC (SDEX) → EURC (AMM) → BTC (SDEX)')
    ).toBeInTheDocument();

    // Split Route tab
    await user.click(screen.getByRole('tab', { name: 'Split Route' }));
    expect(
      await screen.findByRole('heading', { name: 'Split Route', level: 2 })
    ).toBeInTheDocument();
  });

  it('renders and toggles interactive states on the states tab', async () => {
    const user = userEvent.setup();
    render(<RouteVisualizationDemo />);

    await user.click(screen.getByRole('tab', { name: 'States' }));

    expect(
      await screen.findByRole('heading', { name: 'Component States', level: 2 })
    ).toBeInTheDocument();
    expect(screen.getByText('Loading State')).toBeInTheDocument();
    expect(screen.getByText('Error State')).toBeInTheDocument();
    expect(screen.getByText('No Route Found')).toBeInTheDocument();
    expect(screen.getAllByText('No route found').length).toBeGreaterThanOrEqual(1);

    // Toggle error button
    const toggleErrorBtn = screen.getByRole('button', { name: 'Toggle Error' });
    expect(screen.queryByText('Failed to fetch route data')).not.toBeInTheDocument();

    await user.click(toggleErrorBtn);
    expect(screen.getByText('Failed to fetch route data')).toBeInTheDocument();

    await user.click(toggleErrorBtn);
    expect(screen.queryByText('Failed to fetch route data')).not.toBeInTheDocument();

    // Simulate loading button
    const simulateLoadingBtn = screen.getByRole('button', { name: 'Simulate Loading' });
    await user.click(simulateLoadingBtn);
  });
});

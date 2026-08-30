import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import RouteVisualizationDemo from './page';

describe('RouteVisualizationDemo Sandbox (#1298)', () => {
  it('renders the sandbox heading and demo badge', () => {
    render(<RouteVisualizationDemo />);
    expect(
      screen.getByText(/Route Visualization Sandbox/i)
    ).toBeInTheDocument();
    expect(screen.getByText(/Demo Only/i)).toBeInTheDocument();
  });

  it('renders all tab navigation triggers', () => {
    render(<RouteVisualizationDemo />);
    expect(
      screen.getByRole('tab', { name: /Path Scenarios/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('tab', { name: /Split Routing/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('tab', { name: /Edge States/i })
    ).toBeInTheDocument();
  });

  it('switches route presets when clicked', () => {
    render(<RouteVisualizationDemo />);
    const singleHopBtn = screen.getByTestId('preset-single-hop');
    fireEvent.click(singleHopBtn);
    expect(screen.getByText(/Hops: 1/i)).toBeInTheDocument();

    const complexBtn = screen.getByTestId('preset-complex');
    fireEvent.click(complexBtn);
    expect(screen.getByText(/Hops: 3/i)).toBeInTheDocument();
  });

  it('resets sandbox to default multi-hop route on reset button click', () => {
    render(<RouteVisualizationDemo />);
    const singleHopBtn = screen.getByTestId('preset-single-hop');
    fireEvent.click(singleHopBtn);
    expect(screen.getByText(/Hops: 1/i)).toBeInTheDocument();

    const resetBtn = screen.getByTestId('reset-sandbox-btn');
    fireEvent.click(resetBtn);
    expect(screen.getByText(/Hops: 2/i)).toBeInTheDocument();
  });
});

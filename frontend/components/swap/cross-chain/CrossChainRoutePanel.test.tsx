import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { CrossChainRoutePanel } from './CrossChainRoutePanel';

describe('CrossChainRoutePanel', () => {
  it('does not show CCTP rail for uncatalogued pairs', () => {
    render(
      <CrossChainRoutePanel
        sourceChainId="ethereum-sepolia"
        destChainId="bitcoin"
        protocol={null}
        executable={false}
        uncatalogued
      />
    );
    expect(screen.getByTestId('cross-chain-route-panel')).toBeInTheDocument();
    expect(screen.queryByTestId('cctp-route-rail')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('CCTP protocol steps')).not.toBeInTheDocument();
    expect(screen.getByText(/not in the corridor catalog/i)).toBeInTheDocument();
  });

  it('shows CCTP preview rail for catalogued cross-chain corridor', () => {
    render(
      <CrossChainRoutePanel
        sourceChainId="ethereum-sepolia"
        destChainId="stellar"
        protocol="cctp-preview"
        executable={false}
      />
    );
    expect(screen.getByTestId('cctp-route-rail')).toBeInTheDocument();
  });

  it('describes same-chain Stellar routes without CCTP unavailable copy', () => {
    render(
      <CrossChainRoutePanel
        sourceChainId="stellar"
        destChainId="stellar"
        protocol="stellar-native"
        executable
        bridgeUnavailable
      />
    );
    expect(
      screen.getByText(/Same-chain Stellar swap/i)
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/CCTP corridor is listed but not executable/i)
    ).not.toBeInTheDocument();
  });
});

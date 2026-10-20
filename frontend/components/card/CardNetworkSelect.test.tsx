import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';

import {
  CARD_CCTP_SOURCE_NETWORKS,
  CardNetworkSelect,
  DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID,
  cardCctpSourceNetworkForCaip2,
  findCardCctpSourceNetwork,
} from './CardNetworkSelect';
import { cardNetworkFixtures } from './fixtures';

function picker() {
  return screen.getByTestId('card-network-picker');
}

function selectNetwork(value: string) {
  fireEvent.change(picker(), { target: { value } });
}

describe('CardNetworkSelect', () => {
  const onNetworkChange = vi.fn();

  beforeEach(() => {
    onNetworkChange.mockClear();
  });

  describe('source network catalogue', () => {
    it('carries Circle CCTP testnet domain ids for every offered network', () => {
      expect(
        CARD_CCTP_SOURCE_NETWORKS.map((network) => [
          network.id,
          network.circleDomain,
        ]),
      ).toEqual([
        ['ethereum-sepolia', 0],
        ['base-sepolia', 16],
        ['arbitrum-sepolia', 18],
      ]);
    });

    it('keeps Ethereum Sepolia the default and the only executable corridor', () => {
      expect(DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID).toBe('ethereum-sepolia');
      expect(
        CARD_CCTP_SOURCE_NETWORKS.filter((network) => network.executable).map(
          (network) => network.id
        )
      ).toEqual(['ethereum-sepolia']);
    });

    it('resolves a wallet-reported CAIP-2 id back to a listed network', () => {
      expect(
        cardCctpSourceNetworkForCaip2(CARD_CCTP_SOURCE_NETWORKS, 'eip155:84532')
          ?.circleDomain
      ).toBe(16);
      expect(
        cardCctpSourceNetworkForCaip2(CARD_CCTP_SOURCE_NETWORKS, 'eip155:1')
      ).toBeNull();
      expect(
        cardCctpSourceNetworkForCaip2(CARD_CCTP_SOURCE_NETWORKS, null)
      ).toBeNull();
      expect(
        findCardCctpSourceNetwork(CARD_CCTP_SOURCE_NETWORKS, 'missing')
      ).toBeNull();
    });
  });

  describe('rendering', () => {
    it('renders one option per supported CCTP network', () => {
      render(<CardNetworkSelect value="ethereum-sepolia" onNetworkChange={onNetworkChange} />);

      for (const network of CARD_CCTP_SOURCE_NETWORKS) {
        expect(
          screen.getByTestId(`card-network-option-${network.id}`)
        ).toBeInTheDocument();
      }
      expect(picker()).toBeInTheDocument();
    });

    it('shows the CCTP domain of the selected network', () => {
      render(
        <CardNetworkSelect
          value="arbitrum-sepolia"
          onNetworkChange={onNetworkChange}
          networks={cardNetworkFixtures.corridorReady}
        />
      );

      expect(screen.getByTestId('card-network-domain')).toHaveTextContent(
        'CCTP domain 18'
      );
      expect(picker()).toHaveValue('arbitrum-sepolia');
    });

    it('labels a corridor the bridge cannot execute yet, without hiding it', () => {
      render(<CardNetworkSelect value="base-sepolia" onNetworkChange={onNetworkChange} />);

      expect(
        screen.getByTestId('card-network-option-base-sepolia')
      ).toHaveTextContent('Base Sepolia — corridor not enabled');
      expect(screen.getByTestId('card-network-notice')).toHaveTextContent(
        /not wired into the CCTP bridge yet/
      );
      expect(screen.getByTestId('card-network-notice')).toHaveAttribute(
        'role',
        'status'
      );
    });

    it('does not warn when the selected corridor is executable', () => {
      render(<CardNetworkSelect value="ethereum-sepolia" onNetworkChange={onNetworkChange} />);

      expect(screen.getByTestId('card-network-notice')).toHaveTextContent(
        /Funds USDC on Ethereum Sepolia/
      );
      expect(screen.queryByRole('status')).not.toBeInTheDocument();
    });

    it('falls back to the first network for an unknown selection', () => {
      render(<CardNetworkSelect value="not-a-network" onNetworkChange={onNetworkChange} />);

      expect(picker()).toHaveValue(DEFAULT_CARD_CCTP_SOURCE_NETWORK_ID);
      expect(screen.getByTestId('card-network-domain')).toHaveTextContent(
        'CCTP domain 0'
      );
    });

    it('locks the control and reports progress while a switch is in flight', () => {
      render(
        <CardNetworkSelect
          value="base-sepolia"
          onNetworkChange={onNetworkChange}
          switching
        />
      );

      expect(picker()).toBeDisabled();
      expect(screen.getByTestId('card-network-notice')).toHaveTextContent(
        'Switching your wallet to Base Sepolia…'
      );
    });

    it('honours the disabled prop', () => {
      render(
        <CardNetworkSelect value="ethereum-sepolia" onNetworkChange={onNetworkChange} disabled />
      );

      expect(picker()).toBeDisabled();
    });
  });

  describe('network change dispatch', () => {
    it('dispatches the whole network object, domain included', () => {
      render(<CardNetworkSelect value="ethereum-sepolia" onNetworkChange={onNetworkChange} />);

      selectNetwork('arbitrum-sepolia');

      expect(onNetworkChange).toHaveBeenCalledTimes(1);
      expect(onNetworkChange).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'arbitrum-sepolia',
          network: 'eip155:421614',
          circleDomain: 18,
        })
      );
    });

    it('dispatches for a corridor that is not executable yet', () => {
      render(<CardNetworkSelect value="ethereum-sepolia" onNetworkChange={onNetworkChange} />);

      selectNetwork('base-sepolia');

      expect(onNetworkChange).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'base-sepolia', circleDomain: 16 })
      );
    });

    it('stays silent when the change names an unknown network', () => {
      render(
        <CardNetworkSelect
          value="ethereum-sepolia"
          onNetworkChange={onNetworkChange}
          networks={cardNetworkFixtures.corridorReady}
        />
      );

      fireEvent.change(picker(), { target: { value: 'polygon-amoy' } });

      expect(onNetworkChange).not.toHaveBeenCalled();
    });
  });
});

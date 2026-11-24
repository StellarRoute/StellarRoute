import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';

import { CardTopUpModal } from './CardTopUpModal';
import { CARD_CCTP_SOURCE_NETWORKS } from './CardNetworkSelect';
import { cardNetworkFixtures } from './fixtures';

const hex = (value: bigint) => `0x${value.toString(16)}`;

const EVM_ADDRESS = `0x${'a'.repeat(40)}`;
const SEPOLIA = 'eip155:11155111';
const BASE_SEPOLIA = 'eip155:84532';

const mocks = vi.hoisted(() => {
  class WalletAdapterError extends Error {
    code: string;

    constructor(message: string, code: string) {
      super(message);
      this.name = 'WalletAdapterError';
      this.code = code;
    }
  }

  const chainIdHexes: Record<string, string> = {
    'eip155:11155111': '0xaa36a7',
    'eip155:84532': '0x14a34',
    'eip155:421614': '0x66eee',
  };

  const runtime = {
    chainCaip: 'eip155:11155111',
    chainIdHex: '0xaa36a7',
    balances: {} as Record<string, string>,
    native: '0xde0b6b3a7640000',
    gasPrice: '0x3b9aca00',
    session: null as null | {
      account: { address: string };
      network: string;
      isConnected: boolean;
    },
    adapterMode: 'full',
    rejectSwitch: false,
    stayOnOldChain: false,
    wrongChain: false,
    failMethod: null as string | null,
    rpcCalls: [] as string[],
    callParams: [] as unknown[],
    switchCalls: [] as string[],
    async switchNetwork(network: string) {
      runtime.switchCalls.push(network);
      if (runtime.rejectSwitch) {
        throw new WalletAdapterError('User rejected the wallet request', 'user_rejected');
      }
      if (!runtime.stayOnOldChain) {
        runtime.chainCaip = network;
        runtime.chainIdHex = chainIdHexes[network] ?? runtime.chainIdHex;
      }
      return {
        network: runtime.chainCaip,
        matchesExpected: runtime.chainCaip === network,
        expected: network,
      };
    },
    async request(method: string, params?: unknown) {
      runtime.rpcCalls.push(method);
      if (runtime.failMethod === method) {
        throw new Error('MetaMask RPC error');
      }
      if (method === 'eth_chainId') {
        return runtime.wrongChain ? '0x999999' : runtime.chainIdHex;
      }
      if (method === 'eth_call') {
        runtime.callParams.push(params);
        return runtime.balances[runtime.chainCaip] ?? '0x0';
      }
      if (method === 'eth_getBalance') return runtime.native;
      if (method === 'eth_gasPrice') return runtime.gasPrice;
      throw new Error(`unexpected rpc ${method}`);
    },
  };

  return { WalletAdapterError, runtime };
});

vi.mock('@/lib/wallet/adapters', () => ({
  WalletAdapterError: mocks.WalletAdapterError,
  getAdapter: () => {
    if (mocks.runtime.adapterMode === 'none') return undefined;
    const adapter: Record<string, unknown> = {
      id: 'evm-injected',
      getSession: async () => mocks.runtime.session,
    };
    if (mocks.runtime.adapterMode === 'full') {
      adapter.switchNetwork = mocks.runtime.switchNetwork;
    }
    return adapter;
  },
}));

vi.mock('@/lib/wallet/adapters/evm/provider', () => ({
  getInjectedEip1193Provider: () => ({
    request: (args: { method: string; params?: unknown }) =>
      mocks.runtime.request(args.method, args.params),
  }),
  eip1193Request: (_provider: unknown, method: string, params?: unknown) =>
    mocks.runtime.request(method, params),
}));

if (typeof global.ResizeObserver === 'undefined') {
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

type ModalProps = React.ComponentProps<typeof CardTopUpModal>;

function renderModal(overrides: Partial<ModalProps> = {}) {
  const props: ModalProps = {
    open: true,
    amount: '120',
    error: null,
    onAmountChange: vi.fn(),
    onOpenChange: vi.fn(),
    onConfirm: vi.fn(),
    sourceAddress: EVM_ADDRESS,
    ...overrides,
  };
  render(<CardTopUpModal {...props} />);
  return props;
}

function picker() {
  return screen.getByTestId('card-network-picker');
}

function pickNetwork(id: string) {
  fireEvent.change(picker(), { target: { value: id } });
}

function balanceLine() {
  return screen.getByTestId('card-top-up-source-balance');
}

/** The balance row only settles after the provider reads land, so wait for it. */
async function balanceShows(text: string) {
  await waitFor(() => expect(balanceLine()).toHaveTextContent(text));
}

describe('CardTopUpModal', () => {
  beforeEach(() => {
    const runtime = mocks.runtime;
    runtime.chainCaip = SEPOLIA;
    runtime.chainIdHex = '0xaa36a7';
    runtime.balances = {
      [SEPOLIA]: hex(5000000000n),
      [BASE_SEPOLIA]: hex(1234567n),
    };
    runtime.native = '0xde0b6b3a7640000';
    runtime.gasPrice = '0x3b9aca00';
    runtime.session = null;
    runtime.adapterMode = 'full';
    runtime.rejectSwitch = false;
    runtime.stayOnOldChain = false;
    runtime.wrongChain = false;
    runtime.failMethod = null;
    runtime.rpcCalls = [];
    runtime.callParams = [];
    runtime.switchCalls = [];
  });

  describe('keeps the live top-up behaviour', () => {
    it('renders the amount field, confirm action and bridge link', async () => {
      const props = renderModal();

      expect(screen.getByTestId('card-top-up-modal')).toBeInTheDocument();
      expect(screen.getByTestId('card-top-up-amount-input')).toHaveValue('120');

      fireEvent.change(screen.getByTestId('card-top-up-amount-input'), {
        target: { value: '80' },
      });
      expect(props.onAmountChange).toHaveBeenCalledWith('80');

      fireEvent.click(screen.getByTestId('card-top-up-confirm'));
      expect(props.onConfirm).toHaveBeenCalledTimes(1);
    });

    it('keeps the original bridge href while the default network is selected', () => {
      renderModal({ amount: '120' });

      expect(screen.getByTestId('card-top-up-bridge-link')).toHaveAttribute(
        'href',
        '/cross-chain-swap?amount=120&destination=card'
      );
    });

    it('shows the page-owned amount error and drops an unparseable amount', () => {
      renderModal({ amount: 'abc', error: 'Enter a top-up amount greater than 0 USDC.' });

      expect(screen.getByTestId('card-top-up-amount-error')).toHaveTextContent(
        'greater than 0'
      );
      expect(screen.getByTestId('card-top-up-bridge-link')).toHaveAttribute(
        'href',
        '/cross-chain-swap?destination=card'
      );
    });

    it('renders nothing in the DOM until the modal opens', () => {
      renderModal({ open: false });

      expect(screen.queryByTestId('card-top-up-modal')).not.toBeInTheDocument();
    });
  });

  describe('network selector integration', () => {
    it('shows the selector on the default CCTP source network', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      await waitFor(() => expect(picker()).toBeInTheDocument());
      expect(picker()).toHaveValue('ethereum-sepolia');
      expect(screen.getByTestId('card-network-domain')).toHaveTextContent('CCTP domain 0');
    });

    it('asks the connected wallet to switch chains and reports the domain downstream', async () => {
      const onSourceNetworkChange = vi.fn();
      renderModal({
        networks: cardNetworkFixtures.corridorReady,
        onSourceNetworkChange,
      });

      pickNetwork('base-sepolia');

      await waitFor(() =>
        expect(mocks.runtime.switchCalls).toContain(BASE_SEPOLIA)
      );
      expect(onSourceNetworkChange).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'base-sepolia', circleDomain: 16 })
      );
      expect(picker()).toHaveValue('base-sepolia');
    });

    it('reverts the selector when the cardholder rejects the switch', async () => {
      const onSourceNetworkChange = vi.fn();
      renderModal({
        networks: cardNetworkFixtures.corridorReady,
        onSourceNetworkChange,
      });

      mocks.runtime.rejectSwitch = true;
      pickNetwork('base-sepolia');

      expect(await screen.findByTestId('card-network-switch-message')).toHaveTextContent(
        /cancelled — staying on Ethereum Sepolia/
      );
      expect(picker()).toHaveValue('ethereum-sepolia');
      expect(onSourceNetworkChange).not.toHaveBeenCalled();
    });

    it('reverts when the wallet offers no network switching', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      mocks.runtime.adapterMode = 'no-switch';
      pickNetwork('base-sepolia');

      expect(await screen.findByTestId('card-network-switch-message')).toHaveTextContent(
        /cannot switch networks/
      );
      expect(picker()).toHaveValue('ethereum-sepolia');
    });

    it('follows the wallet when it stays on the chain it was already on', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      mocks.runtime.stayOnOldChain = true;
      pickNetwork('base-sepolia');

      expect(await screen.findByTestId('card-network-switch-message')).toHaveTextContent(
        /stayed on Ethereum Sepolia/
      );
      expect(picker()).toHaveValue('ethereum-sepolia');
    });

    it('carries the chosen chain and CCTP domain to the bridge page', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      pickNetwork('base-sepolia');

      expect(await screen.findByTestId('card-top-up-bridge-link')).toHaveAttribute(
        'href',
        '/cross-chain-swap?amount=120&destination=card&source_chain=eip155%3A84532&source_domain=16'
      );
    });

    it('starts from the chain the connected EVM wallet is already on', async () => {
      mocks.runtime.session = {
        account: { address: EVM_ADDRESS },
        network: BASE_SEPOLIA,
        isConnected: true,
      };

      renderModal({ networks: cardNetworkFixtures.corridorReady, sourceAddress: undefined });

      await waitFor(() => expect(picker()).toHaveValue('base-sepolia'));
    });

    it('warns about a corridor the bridge cannot execute yet', async () => {
      renderModal();

      pickNetwork('base-sepolia');

      expect(await screen.findByTestId('card-network-notice')).toHaveTextContent(
        /not wired into the CCTP bridge yet/
      );
      expect(screen.getByTestId('card-network-domain')).toHaveTextContent('CCTP domain 16');
    });
  });

  describe('balance and gas refresh', () => {
    it('reads the source USDC balance and gas on the selected network', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      await balanceShows('5000 USDC');
      expect(mocks.runtime.rpcCalls).toEqual(
        expect.arrayContaining(['eth_chainId', 'eth_call', 'eth_getBalance', 'eth_gasPrice'])
      );
      expect(screen.getByTestId('card-top-up-source-gas')).toHaveTextContent(
        '≤ 0.0003 ETH (have 1)'
      );
    });

    it('pads the wallet address into a balanceOf word', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      await waitFor(() => expect(mocks.runtime.callParams.length).toBeGreaterThan(0));
      expect(mocks.runtime.callParams[0]).toEqual([
        {
          to: cardNetworkFixtures.corridorReady[0].usdcAddress,
          data: `0x70a08231${'0'.repeat(24)}${'a'.repeat(40)}`,
        },
        'latest',
      ]);
    });

    it('re-queries balance and gas after a network switch', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      await balanceShows('5000 USDC');
      const readsBeforeSwitch = mocks.runtime.rpcCalls.filter((m) => m === 'eth_call').length;

      pickNetwork('base-sepolia');

      await waitFor(() =>
        expect(mocks.runtime.rpcCalls.filter((m) => m === 'eth_call').length).toBeGreaterThan(
          readsBeforeSwitch
        )
      );
      await balanceShows('1.234567 USDC');
    });

    it('flags a wallet that is still on another chain', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      await balanceShows('5000 USDC');
      mocks.runtime.wrongChain = true;
      pickNetwork('base-sepolia');

      await waitFor(() =>
        expect(screen.getByTestId('card-top-up-source-status')).toHaveTextContent(
          /not on Base Sepolia yet/
        )
      );
      expect(balanceLine()).toHaveTextContent('—');
    });

    it('warns when the source chain cannot cover bridge gas', async () => {
      mocks.runtime.native = hex(1n);
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      expect(await screen.findByTestId('card-top-up-source-gas-short')).toHaveTextContent(
        /Not enough ETH on Ethereum Sepolia/
      );
    });

    it('skips contract reads for a corridor with no source USDC contract', async () => {
      renderModal();

      await balanceShows('5000 USDC');
      const callsBeforeSwitch = mocks.runtime.rpcCalls.filter((m) => m === 'eth_call').length;

      pickNetwork('base-sepolia');

      expect(await screen.findByTestId('card-top-up-source-status')).toHaveTextContent(
        /Balance and gas preview is unavailable on Base Sepolia/
      );
      expect(
        mocks.runtime.rpcCalls.filter((m) => m === 'eth_call').length
      ).toBe(callsBeforeSwitch);
    });

    it('prompts for a wallet when no source address is known', async () => {
      renderModal({ networks: cardNetworkFixtures.corridorReady, sourceAddress: undefined });

      await waitFor(() =>
        expect(screen.getByTestId('card-top-up-source-status')).toHaveTextContent(
          /Connect an EVM wallet/
        )
      );
      expect(mocks.runtime.rpcCalls).not.toContain('eth_call');
    });

    it('surfaces an rpc failure instead of showing a stale balance', async () => {
      mocks.runtime.failMethod = 'eth_getBalance';
      renderModal({ networks: cardNetworkFixtures.corridorReady });

      expect(await screen.findByTestId('card-top-up-source-status')).toHaveTextContent(
        /Could not read Ethereum Sepolia: MetaMask RPC error/
      );
      expect(balanceLine()).toHaveTextContent('—');
    });

    it('lists every shipped CCTP network as a selectable option', () => {
      renderModal();

      for (const network of CARD_CCTP_SOURCE_NETWORKS) {
        expect(
          screen.getByTestId(`card-network-option-${network.id}`)
        ).toBeInTheDocument();
      }
    });
  });
});

import { getHorizonUrl } from '@/lib/network-endpoints';
import type { WalletNetwork } from '@/lib/wallet/types';

export type BalanceToolResult =
  | {
      kind: 'connect_prompt';
      message: string;
    }
  | {
      kind: 'balances';
      address: string;
      network: WalletNetwork | null;
      summary: string;
      balances: Array<{
        asset: string;
        balance: string;
      }>;
    }
  | {
      kind: 'error';
      message: string;
    };

export interface ReadConnectedAccountBalancesInput {
  connected: boolean;
  address: string | null;
  network?: WalletNetwork | null;
}

export interface HorizonAccountBalanceLine {
  balance: string;
  asset_type: string;
  asset_code?: string;
  asset_issuer?: string;
}

export interface HorizonAccountResponse {
  balances?: HorizonAccountBalanceLine[];
}

function renderAssetLabel(balance: HorizonAccountBalanceLine): string {
  if (balance.asset_type === 'native') {
    return 'XLM';
  }

  const code = balance.asset_code ?? 'UNKNOWN';
  return code;
}

export function renderBalanceSummary(account: {
  address: string;
  network: WalletNetwork | null;
  balances: HorizonAccountBalanceLine[];
}): string {
  if (!account.balances.length) {
    return `Connected account ${account.address} has no visible balances on ${account.network ?? 'the selected network'}.`;
  }

  const lines = account.balances
    .map((balance) => `${renderAssetLabel(balance)}: ${balance.balance}`)
    .join('\n');

  return `Connected wallet ${account.address} on ${account.network ?? 'the selected network'}\n${lines}`;
}

export async function readConnectedAccountBalances({
  connected,
  address,
  network = null,
}: ReadConnectedAccountBalancesInput): Promise<BalanceToolResult> {
  if (!connected || !address) {
    return {
      kind: 'connect_prompt',
      message: 'Connect your wallet to view the connected account balance.',
    };
  }

  const horizonUrl = getHorizonUrl(network);
  const url = `${horizonUrl.replace(/\/$/, '')}/accounts/${encodeURIComponent(address)}`;

  try {
    const response = await fetch(url, {
      headers: { Accept: 'application/json' },
    });

    if (response.status === 404) {
      const summary = `Connected wallet ${address} on ${network ?? 'the selected network'}\nXLM: 0.0000000`;
      return {
        kind: 'balances',
        address,
        network,
        summary,
        balances: [{ asset: 'XLM', balance: '0.0000000' }],
      };
    }

    if (!response.ok) {
      return {
        kind: 'error',
        message: `Horizon account lookup failed with HTTP ${response.status}.`,
      };
    }

    const body = (await response.json()) as HorizonAccountResponse;
    const balances = (body.balances ?? []).map((balance) => ({
      asset: renderAssetLabel(balance),
      balance: balance.balance,
    }));

    const summary = renderBalanceSummary({
      address,
      network,
      balances: body.balances ?? [],
    });

    return {
      kind: 'balances',
      address,
      network,
      summary,
      balances,
    };
  } catch (error) {
    return {
      kind: 'error',
      message:
        error instanceof Error
          ? `Balance lookup failed: ${error.message}`
          : 'Balance lookup failed.',
    };
  }
}

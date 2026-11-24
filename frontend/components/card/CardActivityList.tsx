'use client';

import {
  type CardActivityItem,
  formatUsdcSigned,
  stellarExpertUrl,
  evmExplorerUrl,
} from '../../lib/card/activity';

export interface CardActivityListProps {
  items: CardActivityItem[];
}

export function CardActivityList({ items }: CardActivityListProps) {
  if (!items.length) {
    return (
      <div data-testid="card-activity-empty" className="card-activity-empty">
        No activity yet
      </div>
    );
  }

  return (
    <ul data-testid="card-activity-list" className="card-activity-list">
      {items.map((item) => (
        <li
          key={item.id}
          data-testid={`activity-${item.type}`}
          data-type={item.type}
          className={`card-activity-item card-activity-item--${item.type}`}
        >
          {item.type === 'cctp_deposit' ? (
            <>
              <span data-testid="badge-incoming" aria-label="incoming">
                ↓ green incoming
              </span>
              <span data-testid="cctp-amount">
                {formatUsdcSigned(item.usdcAmount)}
              </span>
              <span data-testid="evm-chain-badge">{item.sourceChain}</span>
              {item.stellarTxHash && (
                <a
                  data-testid="stellar-expert-link"
                  href={stellarExpertUrl(item.stellarTxHash)}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Stellar Expert
                </a>
              )}
              {item.evmTxHash && (
                <a
                  data-testid="evm-explorer-link"
                  href={evmExplorerUrl(item.sourceChain, item.evmTxHash)}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Explorer
                </a>
              )}
            </>
          ) : (
            <>
              <span data-testid="badge-pos" aria-label="pos debit">
                🛒
              </span>
              <span data-testid="merchant-name">{item.merchantName}</span>
              <span data-testid="fiat-breakdown">
                {item.fiatAmount.toFixed(2)} {item.fiatCurrency}
              </span>
              <span data-testid="pos-usdc">
                {formatUsdcSigned(item.usdcAmount)}
              </span>
            </>
          )}
        </li>
      ))}
    </ul>
  );
}
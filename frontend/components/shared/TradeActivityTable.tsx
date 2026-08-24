// frontend/components/shared/TradeActivityTable.tsx
import React from 'react';
import { useTradeActivity } from '../../hooks/useTradeActivity';
// Adjusted from ../../../lib/trade-format to ../../lib/trade-format
import { truncateTxHash, formatTradeTimestamp, formatTradeAmount, stellarExplorerUrl } from '../../lib/trade-format';
import { TradeRecord } from '../../types/trade';

interface TradeActivityTableProps {
  address?: string;
  initialData?: TradeRecord[];
}

export const TradeActivityTable: React.FC<TradeActivityTableProps> = ({ address, initialData }) => {
  const {
    data,
    page,
    totalPages,
    setPage,
    handleSort,
    sortField,
    sortDirection,
    isLoading,
    isEmpty,
  } = useTradeActivity({ address, initialData });

  if (isLoading) return <div data-testid="loading-state">Loading trade activity...</div>;
  if (isEmpty) return <div data-testid="empty-state">No trade activity found.</div>;

  return (
    <div className="trade-activity-container">
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr>
            <th onClick={() => handleSort('timestamp')} style={{ cursor: 'pointer' }}>
              Date/Time {sortField === 'timestamp' ? (sortDirection === 'asc' ? '▲' : '▼') : ''}
            </th>
            <th onClick={() => handleSort('action')} style={{ cursor: 'pointer' }}>
              Action {sortField === 'action' ? (sortDirection === 'asc' ? '▲' : '▼') : ''}
            </th>
            <th onClick={() => handleSort('amount')} style={{ cursor: 'pointer' }}>
              Amount {sortField === 'amount' ? (sortDirection === 'asc' ? '▲' : '▼') : ''}
            </th>
            <th onClick={() => handleSort('asset')} style={{ cursor: 'pointer' }}>
              Asset {sortField === 'asset' ? (sortDirection === 'asc' ? '▲' : '▼') : ''}
            </th>
            <th>Tx Hash</th>
          </tr>
        </thead>
        <tbody>
          {data.map((trade) => (
            <tr key={trade.id} data-testid="trade-row">
              <td>{formatTradeTimestamp(trade.timestamp)}</td>
              <td>{trade.action}</td>
              <td>{formatTradeAmount(trade.amount)}</td>
              <td>{trade.asset}</td>
              <td>
                <a href={stellarExplorerUrl(trade.txHash)} target="_blank" rel="noopener noreferrer">
                  {truncateTxHash(trade.txHash)}
                </a>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div style={{ marginTop: '1rem', display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
        <button onClick={() => setPage(page - 1)} disabled={page === 1} data-testid="prev-page">Previous</button>
        <span>Page {page} of {totalPages || 1}</span>
        <button onClick={() => setPage(page + 1)} disabled={page === totalPages || totalPages === 0} data-testid="next-page">Next</button>
      </div>
    </div>
  );
};
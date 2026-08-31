// frontend/components/shared/TradeActivityTable.tsx
import React, { useState } from 'react';
import { useTradeActivity, TradeActivityFilters } from '../../hooks/useTradeActivity';
// Adjusted from ../../../lib/trade-format to ../../lib/trade-format
import { truncateTxHash, formatTradeTimestamp, formatTradeAmount, stellarExplorerUrl } from '../../lib/trade-format';
import { TradeRecord } from '../../types/trade';
import { useFeatureFlag } from '../../hooks/useFeatureFlag';

interface TradeActivityTableProps {
  address?: string;
  initialData?: TradeRecord[];
}

export const TradeActivityTable: React.FC<TradeActivityTableProps> = ({ address, initialData }) => {
  const { enabled: isHistoryFiltersEnabled } = useFeatureFlag('transaction_history');

  const [filters, setFilters] = useState<TradeActivityFilters>({
    pair: '',
    status: '',
    dateFrom: '',
    dateTo: '',
  });

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
  } = useTradeActivity({
    address,
    initialData,
    filters: isHistoryFiltersEnabled ? filters : undefined,
  });

  const handleResetFilters = () => {
    setFilters({
      pair: '',
      status: '',
      dateFrom: '',
      dateTo: '',
    });
  };

  const hasActiveFilters = Boolean(
    filters.pair ||
    filters.status ||
    filters.dateFrom ||
    filters.dateTo
  );

  return (
    <div className="trade-activity-container">
      {isHistoryFiltersEnabled && (
        <div
          data-testid="history-filters"
          style={{
            display: 'flex',
            flexWrap: 'wrap',
            gap: '1rem',
            alignItems: 'center',
            marginBottom: '1.25rem',
            padding: '1rem',
            borderRadius: '0.5rem',
            backgroundColor: 'rgba(255, 255, 255, 0.05)',
            border: '1px solid rgba(255, 255, 255, 0.1)',
          }}
        >
          {/* Pair / Asset filter */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
            <label htmlFor="filter-pair" style={{ fontSize: '0.875rem', opacity: 0.8 }}>
              Pair / Asset
            </label>
            <input
              id="filter-pair"
              data-testid="filter-pair"
              type="text"
              placeholder="e.g. XLM or USDC"
              value={filters.pair || ''}
              onChange={(e) => setFilters((prev) => ({ ...prev, pair: e.target.value }))}
              style={{
                padding: '0.4rem 0.6rem',
                borderRadius: '0.25rem',
                border: '1px solid rgba(255, 255, 255, 0.2)',
                backgroundColor: 'rgba(0, 0, 0, 0.2)',
                color: 'inherit',
                fontSize: '0.875rem',
              }}
            />
          </div>

          {/* Status / Action filter */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
            <label htmlFor="filter-status" style={{ fontSize: '0.875rem', opacity: 0.8 }}>
              Status / Action
            </label>
            <select
              id="filter-status"
              data-testid="filter-status"
              value={filters.status || ''}
              onChange={(e) => setFilters((prev) => ({ ...prev, status: e.target.value }))}
              style={{
                padding: '0.4rem 0.6rem',
                borderRadius: '0.25rem',
                border: '1px solid rgba(255, 255, 255, 0.2)',
                backgroundColor: 'rgba(0, 0, 0, 0.2)',
                color: 'inherit',
                fontSize: '0.875rem',
              }}
            >
              <option value="">All Statuses</option>
              <option value="SWAP">SWAP</option>
              <option value="BUY">BUY</option>
              <option value="SELL">SELL</option>
              <option value="SEND">SEND</option>
              <option value="RECEIVE">RECEIVE</option>
            </select>
          </div>

          {/* Date From */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
            <label htmlFor="filter-date-from" style={{ fontSize: '0.875rem', opacity: 0.8 }}>
              From Date
            </label>
            <input
              id="filter-date-from"
              data-testid="filter-date-from"
              type="date"
              value={typeof filters.dateFrom === 'string' ? filters.dateFrom : ''}
              onChange={(e) => setFilters((prev) => ({ ...prev, dateFrom: e.target.value }))}
              style={{
                padding: '0.4rem 0.6rem',
                borderRadius: '0.25rem',
                border: '1px solid rgba(255, 255, 255, 0.2)',
                backgroundColor: 'rgba(0, 0, 0, 0.2)',
                color: 'inherit',
                fontSize: '0.875rem',
              }}
            />
          </div>

          {/* Date To */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
            <label htmlFor="filter-date-to" style={{ fontSize: '0.875rem', opacity: 0.8 }}>
              To Date
            </label>
            <input
              id="filter-date-to"
              data-testid="filter-date-to"
              type="date"
              value={typeof filters.dateTo === 'string' ? filters.dateTo : ''}
              onChange={(e) => setFilters((prev) => ({ ...prev, dateTo: e.target.value }))}
              style={{
                padding: '0.4rem 0.6rem',
                borderRadius: '0.25rem',
                border: '1px solid rgba(255, 255, 255, 0.2)',
                backgroundColor: 'rgba(0, 0, 0, 0.2)',
                color: 'inherit',
                fontSize: '0.875rem',
              }}
            />
          </div>

          {/* Reset button */}
          {hasActiveFilters && (
            <div style={{ display: 'flex', alignItems: 'flex-end', paddingTop: '1.25rem' }}>
              <button
                type="button"
                data-testid="filter-reset-button"
                onClick={handleResetFilters}
                style={{
                  padding: '0.4rem 0.8rem',
                  borderRadius: '0.25rem',
                  border: '1px solid rgba(255, 255, 255, 0.2)',
                  backgroundColor: 'rgba(255, 255, 255, 0.1)',
                  color: 'inherit',
                  cursor: 'pointer',
                  fontSize: '0.875rem',
                }}
              >
                Reset Filters
              </button>
            </div>
          )}
        </div>
      )}

      {isLoading ? (
        <div data-testid="loading-state">Loading trade activity...</div>
      ) : isEmpty ? (
        <div data-testid="empty-state">No trade activity found.</div>
      ) : (
        <>
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
        </>
      )}
    </div>
  );
};
// frontend/app/history/page.tsx
'use client';

import React from 'react';
import { HistoryPageClient } from './HistoryPageClient';
// Preserve your existing wallet hook import here. For example:
// import { useWallet } from '../../hooks/useWallet';

export default function HistoryPage() {
  // 1. Keep your existing wallet/account address logic intact
  // const { address } = useWallet();
  const sampleAddress = "GBRPDEJSTXWHLT2YTIU6X7E3E5B5O3N4CUXOAT76O4Q4WUPTFBJMDSZH"; // Fallback/Placeholder

  return (
    <div className="history-page-container" style={{ padding: '2rem' }}>
      <h1 style={{ marginBottom: '1.5rem', fontSize: '1.75rem', fontWeight: 'bold' }}>
        Trade Activity History
      </h1>

      {/* 2. The client shell owns the fetch so the empty state can be illustrated */}
      <HistoryPageClient address={sampleAddress} />
    </div>
  );
}

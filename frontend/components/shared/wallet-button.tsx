'use client';

import { useState } from 'react';
import { QRCodeSVG } from 'qrcode.react';
import { useWallet } from '@/hooks/useWallet';
import { AccountSwitcher } from './account-switcher';

const APP_NETWORK = 'TESTNET';

export function WalletButton() {
  const [showQrCode, setShowQrCode] = useState(false);
  const {
    session,
    availableWallets,
    loading,
    error,
    shortAddress,
    connect,
    disconnect,
    copyAddress,
  } = useWallet();

  const mismatch =
    session.network &&
    session.network.toUpperCase() !== APP_NETWORK.toUpperCase();

  if (!session.isConnected) {
    return (
      <div className="flex flex-col gap-2">
        <div className="flex gap-2">
          {availableWallets.length > 0 ? (
            availableWallets.map((wallet) => (
              <button
                key={wallet.id}
                onClick={() => connect(wallet.id)}
                disabled={loading}
                className="rounded-md border px-3 py-2 text-sm min-h-[44px]"
              >
                {loading ? 'Connecting...' : `Connect ${wallet.label}`}
              </button>
            ))
          ) : (
            <div className="text-sm">
              No supported wallet found. Install Freighter or xBull.
            </div>
          )}
        </div>

        {availableWallets.length === 0 && (
          <div className="text-xs">
            <a
              href="https://www.freighter.app/"
              target="_blank"
              rel="noreferrer"
              className="underline"
            >
              Install Freighter
            </a>{' '}
            |{' '}
            <a
              href="https://wallet.xbull.app/"
              target="_blank"
              rel="noreferrer"
              className="underline"
            >
              Install xBull
            </a>
          </div>
        )}

        {error && <p className="text-sm text-red-500">{error}</p>}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <AccountSwitcher
        onAccountChange={(newAddress) => {
          console.log('Account changed to:', newAddress);
          // This could trigger balance/quote refreshes
          setShowQrCode(false);
        }}
      />

      <div className="flex items-center gap-2">
        <span className="rounded-md border px-3 py-2 text-sm font-mono bg-muted/20">
          {shortAddress}
        </span>

        <button
          onClick={() => setShowQrCode(!showQrCode)}
          className={`rounded-md border px-3 py-2 text-sm font-medium transition-all duration-200 cursor-pointer ${
            showQrCode
              ? 'bg-primary text-primary-foreground border-primary shadow-sm'
              : 'hover:bg-accent hover:text-accent-foreground bg-background'
          }`}
          title={showQrCode ? 'Hide QR Code' : 'Show QR Code'}
          aria-expanded={showQrCode}
        >
          {showQrCode ? 'Hide QR' : 'Show QR'}
        </button>

        <button
          onClick={copyAddress}
          className="rounded-md border px-3 py-2 text-sm hover:bg-accent hover:text-accent-foreground bg-background transition-colors cursor-pointer"
        >
          Copy
        </button>

        <button
          onClick={disconnect}
          className="rounded-md border px-3 py-2 text-sm hover:bg-destructive hover:text-destructive-foreground bg-background transition-colors cursor-pointer"
        >
          Disconnect
        </button>
      </div>

      {showQrCode && session.address && (
        <div className="flex flex-col items-center gap-3 rounded-xl border bg-card p-4 text-card-foreground shadow-md transition-all duration-300 animate-in fade-in slide-in-from-top-2">
          <div className="rounded-lg bg-white p-3 shadow-inner border border-border flex items-center justify-center">
            <QRCodeSVG
              value={session.address}
              size={160}
              level="H"
              includeMargin={true}
            />
          </div>
          <div className="flex flex-col items-center gap-1 text-center w-full max-w-[220px]">
            <span className="text-[10px] uppercase tracking-wider font-semibold text-muted-foreground">
              Public Address
            </span>
            <span className="text-xs font-mono select-all break-all text-foreground/80 leading-relaxed">
              {session.address}
            </span>
          </div>
        </div>
      )}

      <div className="text-sm text-muted-foreground">
        Wallet network: {session.network ?? 'Unknown'}
      </div>

      {mismatch && (
        <div className="text-sm text-yellow-600 font-medium">
          Network mismatch: app is {APP_NETWORK}, wallet is {session.network}
        </div>
      )}

      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  );
}

"use client";

import { useWallet } from "@/hooks/useWallet";
import { AccountSwitcher } from "./account-switcher";

const APP_NETWORK = "TESTNET";

export function WalletButton() {
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
                className="rounded-md border px-3 py-2 text-sm"
              >
                {loading ? "Connecting..." : `Connect ${wallet.label}`}
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
            </a>{" "}
            |{" "}
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
          console.log("Account changed to:", newAddress);
          // This could trigger balance/quote refreshes
        }}
      />
      
      <div className="flex items-center gap-2">
        <span className="rounded-md border px-3 py-2 text-sm">
          {shortAddress}
        </span>

        <button
          onClick={copyAddress}
          className="rounded-md border px-3 py-2 text-sm"
        >
          Copy
        </button>

        <button
          onClick={disconnect}
          className="rounded-md border px-3 py-2 text-sm"
        >
          Disconnect
        </button>
      </div>

      <div className="text-sm">
        Wallet network: {session.network ?? "Unknown"}
      </div>

      {mismatch && (
        <div className="text-sm text-yellow-600">
          Network mismatch: app is {APP_NETWORK}, wallet is {session.network}
        </div>
      )}

      {error && <p className="text-sm text-red-500">{error}</p>}
    </div>
  );
}
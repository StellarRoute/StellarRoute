import {
  requestAccess,
  getAddress,
  getNetworkDetails,
  isAllowed,
  signTransaction,
} from "@stellar/freighter-api";

import type { AvailableWallet, SupportedWallet, WalletSession, Capabilities, Capability, CapabilityStatus } from "./types";

export const WALLET_LABELS: Record<SupportedWallet, string> = {
  freighter: "Freighter",
  xbull: "xBull",
};

export async function getAvailableWallets(): Promise<AvailableWallet[]> {
  const wallets: AvailableWallet[] = [];

  // Freighter
  try {
    const res = await isAllowed();
    wallets.push({
      id: "freighter",
      label: "Freighter",
      installed: res.isAllowed,
    });
  } catch {
    wallets.push({ id: "freighter", label: "Freighter", installed: false });
  }

  // xBull — detected via window.xbull
  const xbullInstalled =
    typeof window !== "undefined" &&
    !!(window as unknown as Record<string, unknown>).xbull;
  wallets.push({ id: "xbull", label: "xBull", installed: xbullInstalled });

  return wallets;
}

export async function connectWallet(
  walletId: SupportedWallet
): Promise<WalletSession> {
  if (walletId === "freighter") {
    const access = await requestAccess();

    if (access.error) {
      throw new Error(access.error.message ?? "Freighter access denied");
    }

    const addressRes = await getAddress();
    if (addressRes.error) {
      throw new Error(addressRes.error.message ?? "Failed to get address");
    }

    const networkRes = await getNetworkDetails();
    if (networkRes.error) {
      throw new Error(networkRes.error.message ?? "Failed to get network");
    }

    return {
      walletId,
      address: addressRes.address,
      network: networkRes.network,
      isConnected: true,
    };
  }

  if (walletId === "xbull") {
    const xbull = (window as unknown as Record<string, unknown>).xbull as
      | { connect: () => Promise<{ publicKey: string }> }
      | undefined;

    if (!xbull) {
      throw new Error("xBull not installed");
    }

    const result = await xbull.connect();
    return {
      walletId,
      address: result.publicKey,
      network: "testnet",
      isConnected: true,
    };
  }

  throw new Error(`Unsupported wallet: ${walletId}`);
}

function getCapabilityResolution(capability: Capability): string {
  switch (capability) {
    case "sign_transaction":
      return "Allow transaction signing in your wallet settings";
    case "view_address":
      return "Allow account access in your wallet settings";
    case "view_network":
      return "Switch to the matching network in your wallet";
    case "request_access":
      return "Reconnect your wallet to grant access";
  }
}

export async function checkWalletCapabilities(
  walletId: SupportedWallet,
  network: string
): Promise<Capabilities> {
  const statuses: CapabilityStatus[] = [];

  if (walletId === "freighter") {
    try {
      const accessRes = await requestAccess();
      statuses.push({
        capability: "request_access",
        allowed: !accessRes.error,
        reason: accessRes.error?.message,
        resolution: accessRes.error ? getCapabilityResolution("request_access") : undefined,
      });
    } catch (e) {
      statuses.push({
        capability: "request_access",
        allowed: false,
        reason: "Failed to check wallet access",
        resolution: getCapabilityResolution("request_access"),
      });
    }

    try {
      const addressRes = await getAddress();
      const hasAddress = !addressRes.error && !!addressRes.address;
      statuses.push({
        capability: "view_address",
        allowed: hasAddress,
        reason: addressRes.error?.message,
        resolution: hasAddress ? undefined : getCapabilityResolution("view_address"),
      });
    } catch (e) {
      statuses.push({
        capability: "view_address",
        allowed: false,
        reason: "Failed to get address",
        resolution: getCapabilityResolution("view_address"),
      });
    }

    try {
      const networkRes = await getNetworkDetails();
      const hasNetwork = !networkRes.error && !!networkRes.network;
      const networkMatch = hasNetwork && networkRes.network === network;
      statuses.push({
        capability: "view_network",
        allowed: networkMatch,
        reason: networkMatch ? undefined : `Wallet on ${networkRes.network}, expected ${network}`,
        resolution: networkMatch ? undefined : "Switch wallet network to match the app",
      });
    } catch (e) {
      statuses.push({
        capability: "view_network",
        allowed: false,
        reason: "Failed to get network details",
        resolution: getCapabilityResolution("view_network"),
      });
    }

    const signCap = statuses.find((s) => s.capability === "view_address");
    const netCap = statuses.find((s) => s.capability === "view_network");
    statuses.push({
      capability: "sign_transaction",
      allowed: signCap?.allowed && netCap?.allowed,
      reason: !signCap?.allowed
        ? "No address available"
        : !netCap?.allowed
          ? "Network mismatch"
          : undefined,
      resolution: !signCap?.allowed || !netCap?.allowed
        ? getCapabilityResolution("sign_transaction")
        : undefined,
    });
  } else if (walletId === "xbull") {
    statuses.push({
      capability: "request_access",
      allowed: true,
    });
    statuses.push({
      capability: "view_address",
      allowed: true,
    });
    statuses.push({
      capability: "view_network",
      allowed: true,
      reason: network === "testnet" ? undefined : "xBull only supports testnet",
      resolution: network !== "testnet" ? "Switch app to testnet" : undefined,
    });
    statuses.push({
      capability: "sign_transaction",
      allowed: network === "testnet",
      reason: network === "testnet" ? undefined : "xBull only supports testnet",
      resolution: network !== "testnet" ? "Switch app to testnet" : undefined,
    });
  }

  return {
    checkedAt: Date.now(),
    statuses,
  };
}

export function disconnectWallet(): WalletSession {
  return {
    walletId: null,
    address: null,
    network: null,
    isConnected: false,
  };
}

export async function signTransactionWithWallet(
  xdr: string,
  walletId: SupportedWallet,
  networkPassphrase?: string
): Promise<string> {
  if (walletId === "freighter") {
    const res = await signTransaction(xdr, { networkPassphrase });
    if (res.error) {
      throw new Error(res.error.message ?? "Transaction signing failed");
    }
    return res.signedTxXdr;
  }

  throw new Error(`Transaction signing not supported for wallet: ${walletId}`);
}

/** Stub for callers that only need the XDR echoed back (e.g. tests / out-of-scope flows) */
export async function signTransactionStub(xdr: string) {
  return {
    ok: false,
    message: "Signing stub only (out of scope)",
    xdr,
  };
}

/** Check if the current wallet address has changed */
export async function checkAddressChange(
  walletId: SupportedWallet,
  currentAddress: string | null
): Promise<string | null> {
  if (!currentAddress) return null;

  try {
    if (walletId === "freighter") {
      const addressRes = await getAddress();
      if (addressRes.error) return null;
      return addressRes.address !== currentAddress ? addressRes.address : null;
    }

    if (walletId === "xbull") {
      // xBull doesn't have a passive address check, would need to reconnect
      return null;
    }
  } catch {
    return null;
  }

  return null;
}

/** Refresh the current session to get updated account info */
export async function refreshWalletSession(
  walletId: SupportedWallet
): Promise<WalletSession> {
  if (walletId === "freighter") {
    const addressRes = await getAddress();
    if (addressRes.error) {
      throw new Error(addressRes.error.message ?? "Failed to get address");
    }

    const networkRes = await getNetworkDetails();
    if (networkRes.error) {
      throw new Error(networkRes.error.message ?? "Failed to get network");
    }

    return {
      walletId,
      address: addressRes.address,
      network: networkRes.network,
      isConnected: true,
    };
  }

  if (walletId === "xbull") {
    const xbull = (window as unknown as Record<string, unknown>).xbull as
      | { connect: () => Promise<{ publicKey: string }> }
      | undefined;

    if (!xbull) {
      throw new Error("xBull not installed");
    }

    const result = await xbull.connect();
    return {
      walletId,
      address: result.publicKey,
      network: "testnet",
      isConnected: true,
    };
  }

  throw new Error(`Unsupported wallet: ${walletId}`);
}

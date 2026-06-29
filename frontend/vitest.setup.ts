import "@testing-library/jest-dom/vitest";
import * as React from "react";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

/** jsdom does not implement matchMedia; components using prefers-reduced-motion need this. */
Object.defineProperty(window, "matchMedia", {
  writable: true,
  configurable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(() => false),
  }),
});

// Ensure localStorage is available in the test environment.
if (
  typeof window.localStorage === "undefined" ||
  typeof window.localStorage.getItem !== "function"
) {
  const localStorageMock = (() => {
    let store: Record<string, string> = {};
    return {
      getItem: (key: string) => (key in store ? store[key] : null),
      setItem: (key: string, value: string) => {
        store[key] = String(value);
      },
      removeItem: (key: string) => {
        delete store[key];
      },
      clear: () => {
        store = {};
      },
    };
  })();

  Object.defineProperty(window, "localStorage", {
    value: localStorageMock,
    configurable: true,
  });
  Object.defineProperty(globalThis, "localStorage", {
    value: localStorageMock,
    configurable: true,
  });
}

// Global mock for WalletProvider/useWallet to prevent test failures across components rendering NetworkMismatchBanner
vi.mock("@/components/providers/wallet-provider", () => ({
  WalletProvider: ({ children }: any) => children,
  useWallet: () => ({
    network: "testnet",
    walletNetwork: "testnet",
    networkMismatch: false,
    walletId: null,
    disconnect: vi.fn(),
    syncMismatch: false,
    resyncWallet: vi.fn(),
    dismissSyncMismatch: vi.fn(),
    isTransactionPending: false,
    isConnected: false,
  }),
}));

// Global mock for next/navigation to support useRouter/useSearchParams/usePathname in tests
vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    prefetch: vi.fn(),
    back: vi.fn(),
  }),
  useSearchParams: () => new URLSearchParams(),
  usePathname: () => "/",
}));


afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
  vi.restoreAllMocks();
});

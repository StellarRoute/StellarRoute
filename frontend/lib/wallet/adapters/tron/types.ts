/** Injected TronLink / TronWeb provider shapes. */

export type TronLinkRequestResult = {
  code?: number;
  message?: string;
};

export type TronWebLike = {
  defaultAddress?: {
    base58?: string;
    hex?: string;
  };
  ready?: boolean;
  fullNode?: { host?: string };
  solidityNode?: { host?: string };
  eventServer?: { host?: string };
  trx?: {
    sign?: (
      transaction: Record<string, unknown>
    ) => Promise<Record<string, unknown>>;
    signMessageV2?: (message: string) => Promise<string>;
    signMessage?: (message: string) => Promise<string>;
  };
};

export type TronLinkProvider = {
  ready?: boolean;
  request?: (args: {
    method: string;
  }) => Promise<TronLinkRequestResult | unknown>;
  tronWeb?: TronWebLike;
};

declare global {
  interface Window {
    tronLink?: TronLinkProvider;
    tronWeb?: TronWebLike;
  }
}

export {};

/**
 * AI Agent Intent Definitions and In-Memory Validator.
 *
 * Provides strongly-typed intent representations for the non-custodial
 * conversational layer. Amounts are decimal strings; assets use canonical
 * identifier codes (e.g. "XLM", "USDC:GA5Z..."). No network calls.
 */

export type IntentType =
  | 'convert'
  | 'send'
  | 'receive'
  | 'bridge'
  | 'offramp'
  | 'subscribe'
  | 'balance';

/** Swap between Stellar assets (SDEX or Soroban AMM). */
export interface ConvertIntent {
  type: 'convert';
  fromAsset: string;
  toAsset: string;
  amount: string;
  slippageTolerance?: string;
}

/** Direct transfer of assets to a destination address. */
export interface SendIntent {
  type: 'send';
  recipient: string;
  asset: string;
  amount: string;
  memo?: string;
}

/** Payment request / receive QR representation. */
export interface ReceiveIntent {
  type: 'receive';
  asset: string;
  amount?: string;
}

/** Cross-chain bridge transfer (Circle CCTP). */
export interface BridgeIntent {
  type: 'bridge';
  sourceAsset: string;
  sourceChain: string;
  destinationAsset: string;
  destinationChain: string;
  amount: string;
  recipient?: string;
}

/** Offramp crypto to fiat (e.g. USDC to NGN via Paycrest). */
export interface OfframpIntent {
  type: 'offramp';
  sourceAsset: string;
  fiatCurrency: string;
  amount: string;
  destinationAccount?: string;
}

/** Recurring subscription schedule setup. */
export interface SubscribeIntent {
  type: 'subscribe';
  payee: string;
  asset: string;
  amount: string;
  interval: 'daily' | 'weekly' | 'monthly' | 'yearly' | string;
  maxPayments: number;
}

/** Read-only balance inquiry. */
export interface BalanceIntent {
  type: 'balance';
  asset?: string;
  account?: string;
}

export type AgentIntent =
  | ConvertIntent
  | SendIntent
  | ReceiveIntent
  | BridgeIntent
  | OfframpIntent
  | SubscribeIntent
  | BalanceIntent;

export class IntentValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'IntentValidationError';
  }
}

function isNonEmptyString(val: unknown): val is string {
  return typeof val === 'string' && val.trim().length > 0;
}

/**
 * Validates and parses raw intent input into a strongly-typed AgentIntent.
 * Throws IntentValidationError if any required fields are missing or invalid.
 */
export function parseIntent(raw: unknown): AgentIntent {
  if (!raw || typeof raw !== 'object') {
    throw new IntentValidationError('Intent must be a non-null object');
  }

  const obj = raw as Record<string, unknown>;
  const type = obj.type;

  if (typeof type !== 'string') {
    throw new IntentValidationError('Intent requires a "type" discriminator');
  }

  switch (type) {
    case 'convert': {
      if (!isNonEmptyString(obj.fromAsset)) {
        throw new IntentValidationError('convert intent requires non-empty "fromAsset"');
      }
      if (!isNonEmptyString(obj.toAsset)) {
        throw new IntentValidationError('convert intent requires non-empty "toAsset"');
      }
      if (!isNonEmptyString(obj.amount)) {
        throw new IntentValidationError('convert intent requires non-empty "amount"');
      }
      return {
        type: 'convert',
        fromAsset: obj.fromAsset.trim(),
        toAsset: obj.toAsset.trim(),
        amount: obj.amount.trim(),
        slippageTolerance: typeof obj.slippageTolerance === 'string' ? obj.slippageTolerance : undefined,
      };
    }

    case 'send': {
      if (!isNonEmptyString(obj.recipient)) {
        throw new IntentValidationError('send intent requires non-empty "recipient"');
      }
      if (!isNonEmptyString(obj.asset)) {
        throw new IntentValidationError('send intent requires non-empty "asset"');
      }
      if (!isNonEmptyString(obj.amount)) {
        throw new IntentValidationError('send intent requires non-empty "amount"');
      }
      return {
        type: 'send',
        recipient: obj.recipient.trim(),
        asset: obj.asset.trim(),
        amount: obj.amount.trim(),
        memo: typeof obj.memo === 'string' ? obj.memo : undefined,
      };
    }

    case 'receive': {
      if (!isNonEmptyString(obj.asset)) {
        throw new IntentValidationError('receive intent requires non-empty "asset"');
      }
      return {
        type: 'receive',
        asset: obj.asset.trim(),
        amount: typeof obj.amount === 'string' && obj.amount.trim().length > 0 ? obj.amount.trim() : undefined,
      };
    }

    case 'bridge': {
      if (!isNonEmptyString(obj.sourceAsset)) {
        throw new IntentValidationError('bridge intent requires non-empty "sourceAsset"');
      }
      if (!isNonEmptyString(obj.sourceChain)) {
        throw new IntentValidationError('bridge intent requires non-empty "sourceChain"');
      }
      if (!isNonEmptyString(obj.destinationAsset)) {
        throw new IntentValidationError('bridge intent requires non-empty "destinationAsset"');
      }
      if (!isNonEmptyString(obj.destinationChain)) {
        throw new IntentValidationError('bridge intent requires non-empty "destinationChain"');
      }
      if (!isNonEmptyString(obj.amount)) {
        throw new IntentValidationError('bridge intent requires non-empty "amount"');
      }
      return {
        type: 'bridge',
        sourceAsset: obj.sourceAsset.trim(),
        sourceChain: obj.sourceChain.trim(),
        destinationAsset: obj.destinationAsset.trim(),
        destinationChain: obj.destinationChain.trim(),
        amount: obj.amount.trim(),
        recipient: typeof obj.recipient === 'string' ? obj.recipient : undefined,
      };
    }

    case 'offramp': {
      if (!isNonEmptyString(obj.sourceAsset)) {
        throw new IntentValidationError('offramp intent requires non-empty "sourceAsset"');
      }
      if (!isNonEmptyString(obj.fiatCurrency)) {
        throw new IntentValidationError('offramp intent requires non-empty "fiatCurrency"');
      }
      if (!isNonEmptyString(obj.amount)) {
        throw new IntentValidationError('offramp intent requires non-empty "amount"');
      }
      return {
        type: 'offramp',
        sourceAsset: obj.sourceAsset.trim(),
        fiatCurrency: obj.fiatCurrency.trim(),
        amount: obj.amount.trim(),
        destinationAccount: typeof obj.destinationAccount === 'string' ? obj.destinationAccount : undefined,
      };
    }

    case 'subscribe': {
      if (!isNonEmptyString(obj.payee)) {
        throw new IntentValidationError('subscribe intent requires non-empty "payee"');
      }
      if (!isNonEmptyString(obj.asset)) {
        throw new IntentValidationError('subscribe intent requires non-empty "asset"');
      }
      if (!isNonEmptyString(obj.amount)) {
        throw new IntentValidationError('subscribe intent requires non-empty "amount"');
      }
      if (!isNonEmptyString(obj.interval)) {
        throw new IntentValidationError('subscribe intent requires non-empty "interval"');
      }
      if (typeof obj.maxPayments !== 'number' || isNaN(obj.maxPayments) || obj.maxPayments < 1) {
        throw new IntentValidationError('subscribe intent requires positive number for "maxPayments"');
      }
      return {
        type: 'subscribe',
        payee: obj.payee.trim(),
        asset: obj.asset.trim(),
        amount: obj.amount.trim(),
        interval: obj.interval.trim(),
        maxPayments: obj.maxPayments,
      };
    }

    case 'balance': {
      return {
        type: 'balance',
        asset: typeof obj.asset === 'string' && obj.asset.trim().length > 0 ? obj.asset.trim() : undefined,
        account: typeof obj.account === 'string' && obj.account.trim().length > 0 ? obj.account.trim() : undefined,
      };
    }

    default:
      throw new IntentValidationError(`Unsupported intent type: "${String(type)}"`);
  }
}

/**
 * Validates raw intent input without throwing, returning a result object.
 */
export function validateIntent(
  raw: unknown,
): { success: true; data: AgentIntent } | { success: false; error: string } {
  try {
    const data = parseIntent(raw);
    return { success: true, data };
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : 'Unknown validation error',
    };
  }
}

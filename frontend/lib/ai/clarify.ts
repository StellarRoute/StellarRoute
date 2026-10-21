import {
  parseIntent,
  type Intent,
  type IntentType,
  type ParseResult,
} from './parse';

/**
 * AI-30: one clarification question when the parse is incomplete.
 *
 * The deterministic parser in `./parse` only answers "is this sentence
 * complete?". This module keeps the half-finished sentence around so the next
 * user message can fill the single missing field, then re-parses the completed
 * sentence with the same parser. Nothing here touches the network, a wallet,
 * or a live trading surface: the returned intent still has to be validated
 * (AI-12) and confirmed before anything is signed.
 */

/** A single slot of an intent sentence the parser can leave empty. */
export type ClarificationField =
  | 'amount'
  | 'asset'
  | 'destination'
  | 'source'
  | 'recipient'
  | 'currency'
  | 'recurrence';

/** Values captured so far, keyed by the slot they belong to. */
export type ClarificationValues = Partial<Record<ClarificationField, string>>;

/** The half-finished sentence, kept between two user messages. */
export interface PendingClarification {
  /** Intent the opening sentence was heading for. */
  type: IntentType;
  /** The sentence that opened the clarification, kept for context. */
  original: string;
  /** Values already read out of that sentence. */
  values: ClarificationValues;
  /** Slot order the parser grammar expects for this intent. */
  order: ClarificationField[];
  /** Slots still empty, in the order they will be asked about. */
  missing: ClarificationField[];
  /** The single slot the last question asked about. */
  asked: ClarificationField;
}

/** A turn that asks one question and exposes no intent. */
export interface ClarificationTurn {
  kind: 'clarification';
  message: string;
  /** The slot this question is about, or `null` when nothing is pending. */
  field: ClarificationField | null;
  /** Question to ask next, or `null` when the pending question was cleared. */
  pending: PendingClarification | null;
}

/** A turn whose fields are all filled and whose intent may be validated. */
export interface IntentTurn {
  kind: 'intent';
  intent: Intent;
  pending: null;
}

export type ClarifyResult = ClarificationTurn | IntentTurn;

type QuestionCopy = (values: ClarificationValues) => string;

interface IntentGrammar {
  /** Slot order the parser expects; the first slot is asked first. */
  order: ClarificationField[];
  /** Canonical sentence rebuilt from the filled slots. */
  render: (values: ClarificationValues) => string;
  question: Partial<Record<ClarificationField, QuestionCopy>>;
}

const FALLBACK_QUESTIONS: Record<ClarificationField, string> = {
  amount: 'How much do you want to trade?',
  asset: 'Which asset do you want to trade?',
  destination: 'Which asset do you want to receive?',
  source: 'Which chain are you bridging from?',
  recipient: 'Which Stellar address should receive it?',
  currency: 'Which currency do you want to cash out to?',
  recurrence: 'How often should it be paid?',
};

function amountOf(asset?: string): string {
  return asset ? ` ${asset}` : '';
}

const GRAMMARS: Record<IntentType, IntentGrammar> = {
  swap: {
    order: ['amount', 'asset', 'destination'],
    render: (v) => `swap ${v.amount} ${v.asset} to ${v.destination}`,
    question: {
      amount: (v) => `How much${amountOf(v.asset)} do you want to swap?`,
      asset: () => 'Which asset do you want to swap?',
      destination: (v) =>
        `Which asset do you want to swap ${v.amount} ${v.asset} for?`,
    },
  },
  send: {
    order: ['amount', 'asset', 'recipient'],
    render: (v) => `send ${v.amount} ${v.asset} to ${v.recipient}`,
    question: {
      amount: (v) => `How much${amountOf(v.asset)} do you want to send?`,
      asset: () => 'Which asset do you want to send?',
      recipient: (v) =>
        `Which Stellar address should receive ${v.amount} ${v.asset}?`,
    },
  },
  receive: {
    order: [],
    render: () => 'receive',
    question: {},
  },
  bridge: {
    order: ['amount', 'asset', 'destination'],
    render: (v) => `bridge ${v.amount} ${v.asset} to ${v.destination}`,
    question: {
      amount: (v) => `How much${amountOf(v.asset)} do you want to bridge?`,
      asset: () => 'Which asset do you want to bridge?',
      destination: (v) =>
        `Which chain do you want to bridge ${v.amount} ${v.asset} to?`,
      source: (v) =>
        `Which chain do you want to bridge ${v.amount} ${v.asset} from?`,
    },
  },
  cash_out: {
    order: ['amount', 'asset', 'currency'],
    render: (v) => `cash out ${v.amount} ${v.asset} to ${v.currency}`,
    question: {
      amount: (v) => `How much${amountOf(v.asset)} do you want to cash out?`,
      asset: () => 'Which asset do you want to cash out?',
      currency: () => 'Which currency do you want to cash out to?',
    },
  },
  pay: {
    order: ['amount', 'asset', 'recurrence', 'recipient'],
    render: (v) =>
      `pay ${v.amount} ${v.asset} ${v.recurrence} to ${v.recipient}`,
    question: {
      amount: (v) => `How much${amountOf(v.asset)} do you want to pay?`,
      asset: () => 'Which asset do you want to pay?',
      recurrence: (v) => `How often do you want to pay ${v.amount} ${v.asset}?`,
      recipient: (v) =>
        `Which Stellar address should receive ${v.amount} ${v.asset}?`,
    },
  },
};

/** Opening verbs, in the order the parser recognises them. */
const VERBS: { pattern: RegExp; type: IntentType }[] = [
  { pattern: /^(?:swap|convert)\b/i, type: 'swap' },
  { pattern: /^send\b/i, type: 'send' },
  { pattern: /^receive\b/i, type: 'receive' },
  { pattern: /^bridge\b/i, type: 'bridge' },
  { pattern: /^cash[\s-]?out\b/i, type: 'cash_out' },
  { pattern: /^pay\b/i, type: 'pay' },
];

const AMOUNT = String.raw`\d+(?:\.\d+)?`;
const CODE = String.raw`[A-Za-z0-9._$-]+`;
const CODE_PATTERN = new RegExp(`^${CODE}$`);
const AMOUNT_PATTERN = new RegExp(`^${AMOUNT}$`);
const AMOUNT_IN_TEXT = new RegExp(AMOUNT);
const ASSET_AFTER_AMOUNT = new RegExp(`${AMOUNT}\\s+(${CODE})`);
const TOKEN_AFTER_TO = new RegExp(String.raw`\bto\s+(${CODE})`, 'i');
const TOKEN_AFTER_FROM = new RegExp(String.raw`\bfrom\s+(${CODE})`, 'i');
const RECURRENCE_IN_TEXT = /\b(monthly|weekly|yearly)\b/i;
const RECURRENCE_ANSWER = /^(monthly|weekly|yearly)$/i;
const RECIPIENT = /^G\w{3,}$/i;

/**
 * Chat words that must never be read back into an intent as a value, so a new
 * sentence clears the pending question instead of filling it.
 */
const NOT_A_VALUE = new Set(
  (
    'a about again all am an and any are as at auto be been bridge but by ' +
    'can cancel cash confirm convert could did do does done exchange for ' +
    'from get give got has have hello help hey hi how into is it just like ' +
    'look make me more my nevermind no now of off ok okay on once or out ' +
    'over pay please receive see send show so some stop swap that then ' +
    'there this thanks to trade try us want was we what whats when where ' +
    'which who why will with would yes you your'
  ).split(' ')
);

/**
 * Lower-case chain and fiat slugs the parser's own examples use. Only used to
 * accept an answer, never to reject a complete sentence.
 */
const KNOWN_SLUGS = new Set(
  (
    'arbitrum avalanche base bitcoin bnb bsc cad chf cny dkk egp eth ' +
    'ethereum eur gbp ghs idr inr jpy kes linea matic mxn ngn naira nok ' +
    'nzd optimism php polygon ron rub scroll sek sepolia solana stellar try ' +
    'tron usd xlm zar zksync'
  ).split(' ')
);

function isIntent(result: ParseResult): result is Intent {
  return 'type' in result;
}

function isFilled(value: string | undefined): boolean {
  return typeof value === 'string' && value.trim() !== '';
}

/**
 * A code-like answer: an asset, a chain, or a fiat currency. Plain lower-case
 * words are refused so a new sentence is never pasted into an intent.
 */
function isCodeAnswer(value: string): boolean {
  if (NOT_A_VALUE.has(value.toLowerCase())) return false;
  if (/\d/.test(value)) return true;
  if (value !== value.toLowerCase()) return true;
  return KNOWN_SLUGS.has(value.toLowerCase());
}

function missingFields(
  order: ClarificationField[],
  values: ClarificationValues
): ClarificationField[] {
  return order.filter((field) => !isFilled(values[field]));
}

function questionFor(
  type: IntentType,
  field: ClarificationField,
  values: ClarificationValues
): string {
  const copy = GRAMMARS[type].question[field];
  return copy ? copy(values) : FALLBACK_QUESTIONS[field];
}

function matchVerb(text: string): { type: IntentType; rest: string } | null {
  for (const verb of VERBS) {
    const matched = verb.pattern.exec(text);
    if (matched) {
      return { type: verb.type, rest: text.slice(matched[0].length) };
    }
  }
  return null;
}

function leadingToken(rest: string): string | undefined {
  const [first = ''] = rest.trim().split(/\s+/);
  if (AMOUNT_PATTERN.test(first)) return undefined;
  if (!CODE_PATTERN.test(first)) return undefined;
  return NOT_A_VALUE.has(first.toLowerCase()) ? undefined : first;
}

function extractValues(type: IntentType, rest: string): ClarificationValues {
  const values: ClarificationValues = {};

  const amount = AMOUNT_IN_TEXT.exec(rest);
  if (amount) {
    values.amount = amount[0];
  }

  const asset = ASSET_AFTER_AMOUNT.exec(rest)?.[1] ?? leadingToken(rest);
  if (asset && !AMOUNT_PATTERN.test(asset) && CODE_PATTERN.test(asset)) {
    values.asset = asset;
  }

  const target = TOKEN_AFTER_TO.exec(rest)?.[1];
  if (
    target &&
    CODE_PATTERN.test(target) &&
    !NOT_A_VALUE.has(target.toLowerCase())
  ) {
    if (type === 'cash_out') {
      values.currency = target;
    } else if (type === 'send' || type === 'pay') {
      if (RECIPIENT.test(target)) values.recipient = target;
    } else {
      values.destination = target;
    }
  }

  const source = TOKEN_AFTER_FROM.exec(rest)?.[1];
  if (
    source &&
    CODE_PATTERN.test(source) &&
    !NOT_A_VALUE.has(source.toLowerCase())
  ) {
    values.source = source;
  }

  const recurrence = RECURRENCE_IN_TEXT.exec(rest);
  if (recurrence) {
    values.recurrence = recurrence[1].toLowerCase();
  }

  return values;
}

function orderFor(type: IntentType, rest: string): ClarificationField[] {
  if (type === 'bridge' && /\bfrom\b/i.test(rest)) {
    return ['amount', 'asset', 'source'];
  }
  return GRAMMARS[type].order;
}

function renderSentence(
  type: IntentType,
  values: ClarificationValues,
  order: ClarificationField[]
): string {
  if (type === 'bridge' && order.includes('source')) {
    return `bridge ${values.amount} ${values.asset} from ${values.source}`;
  }
  return GRAMMARS[type].render(values);
}

/** Reads the half-finished sentence, or `null` when it is not one. */
function openPending(text: string): PendingClarification | null {
  const verb = matchVerb(text);
  if (!verb) return null;

  const order = orderFor(verb.type, verb.rest);
  const values = extractValues(verb.type, verb.rest);
  const missing = missingFields(order, values);
  if (missing.length === 0) return null;

  return {
    type: verb.type,
    original: text,
    values,
    order,
    missing,
    asked: missing[0],
  };
}

/** Reads a one-token answer, or `null` when the message is not an answer. */
function answerFor(field: ClarificationField, text: string): string | null {
  const value = text.trim().replace(/[.,!?;:]+$/, '');
  if (/\s/.test(value) || !CODE_PATTERN.test(value)) return null;

  if (field === 'amount') {
    return AMOUNT_PATTERN.test(value) ? value : null;
  }
  if (field === 'recipient') {
    return RECIPIENT.test(value) ? value : null;
  }
  if (field === 'recurrence') {
    return RECURRENCE_ANSWER.test(value) ? value : null;
  }
  return isCodeAnswer(value) ? value : null;
}

function continuePending(
  pending: PendingClarification,
  field: ClarificationField,
  value: string
): ClarifyResult {
  const values = { ...pending.values, [field]: value };
  const missing = missingFields(pending.order, values);

  if (missing.length > 0) {
    const asked = missing[0];
    return {
      kind: 'clarification',
      message: questionFor(pending.type, asked, values),
      field: asked,
      pending: { ...pending, values, missing, asked },
    };
  }

  const parsed = parseIntent(
    renderSentence(pending.type, values, pending.order)
  );
  if (isIntent(parsed)) {
    return { kind: 'intent', intent: parsed, pending: null };
  }

  // The parser is the only thing allowed to call an intent complete, so an
  // answer it cannot express is dropped and the same question is asked again.
  return reask(pending);
}

function freshTurn(sentence: string): ClarifyResult {
  const parsed = parseIntent(sentence);
  if (isIntent(parsed)) {
    return { kind: 'intent', intent: parsed, pending: null };
  }

  const pending = openPending(sentence);
  if (!pending) {
    return {
      kind: 'clarification',
      message: parsed.message,
      field: null,
      pending: null,
    };
  }

  return {
    kind: 'clarification',
    message: questionFor(pending.type, pending.asked, pending.values),
    field: pending.asked,
    pending,
  };
}

function reask(pending: PendingClarification): ClarifyResult {
  return {
    kind: 'clarification',
    message: questionFor(pending.type, pending.asked, pending.values),
    field: pending.asked,
    pending,
  };
}

/**
 * Resolves one user message against the question that is still open.
 *
 * A message that answers the open question fills that slot, and the completed
 * sentence is re-parsed with `./parse`. Any other message is read as a new
 * sentence, which clears the pending question. An answer the parser cannot
 * express keeps the question open instead of losing the sentence.
 */
export function resolveTurn(
  text: string,
  pending: PendingClarification | null = null
): ClarifyResult {
  const sentence = text.trim();
  if (sentence === '') {
    return pending ? reask(pending) : freshTurn(sentence);
  }

  if (pending) {
    const answer = answerFor(pending.asked, sentence);
    if (answer !== null) {
      return continuePending(pending, pending.asked, answer);
    }
  }

  return freshTurn(sentence);
}

/** True only when every field is filled and the intent may be validated. */
export function isReadyForValidation(result: ClarifyResult): boolean {
  return result.kind === 'intent';
}

/**
 * The intent to hand to validate (AI-12), or `null` while any field is still
 * missing. No intent is exposed until the parse is complete.
 */
export function intentForValidation(result: ClarifyResult): Intent | null {
  return result.kind === 'intent' ? result.intent : null;
}

/** Slots still waiting for an answer; empty when nothing is pending. */
export function missingClarificationFields(
  pending: PendingClarification | null
): ClarificationField[] {
  return pending ? [...pending.missing] : [];
}

export interface Clarifier {
  /** Feeds one user message in and keeps the open question for the next. */
  submit(text: string): ClarifyResult;
  /** Drops the open question, e.g. when the transcript is cleared. */
  reset(): void;
  /** The question waiting for an answer, if any. */
  readonly pending: PendingClarification | null;
}

/** Stateful wrapper around {@link resolveTurn} for the chat shell. */
export function createClarifier(): Clarifier {
  let pending: PendingClarification | null = null;

  return {
    submit(text: string): ClarifyResult {
      const result = resolveTurn(text, pending);
      pending = result.pending;
      return result;
    },
    reset(): void {
      pending = null;
    },
    get pending(): PendingClarification | null {
      return pending;
    },
  };
}

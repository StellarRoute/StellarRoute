export const DEFAULT_CARD_NICKNAME = 'Stellar Virtual Debit';
export const MAX_CARD_NICKNAME_LENGTH = 24;
export const CARD_NICKNAME_STORAGE_PREFIX = 'stellar_route_card_nickname';

export function cardNicknameStorageKey(stellarAddress: string): string {
  const address = stellarAddress.trim();
  if (address.length === 0) {
    throw new TypeError(
      'Card nicknames must be scoped to a non-empty Stellar public key.'
    );
  }
  return `${CARD_NICKNAME_STORAGE_PREFIX}:${address}`;
}

export function sanitizeCardNickname(nickname: string): string {
  return Array.from(nickname.replace(/<[^>]*>/g, '').trim())
    .slice(0, MAX_CARD_NICKNAME_LENGTH)
    .join('');
}

export function loadCardNickname(stellarAddress: string): string | null {
  if (typeof window === 'undefined') return null;

  const stored = window.localStorage.getItem(
    cardNicknameStorageKey(stellarAddress)
  );
  if (stored === null) return null;

  const nickname = sanitizeCardNickname(stored);
  return nickname.length > 0 ? nickname : null;
}

export function getCardNickname(stellarAddress: string): string {
  return loadCardNickname(stellarAddress) ?? DEFAULT_CARD_NICKNAME;
}

export function saveCardNickname(
  stellarAddress: string,
  nickname: string
): string {
  const key = cardNicknameStorageKey(stellarAddress);
  if (nickname.length === 0) {
    if (typeof window !== 'undefined') window.localStorage.removeItem(key);
    return DEFAULT_CARD_NICKNAME;
  }

  const sanitized = sanitizeCardNickname(nickname);
  if (sanitized.length === 0) {
    throw new TypeError('Card nicknames cannot contain only whitespace.');
  }
  if (typeof window !== 'undefined') {
    window.localStorage.setItem(key, sanitized);
  }
  return sanitized;
}

export function resetCardNickname(stellarAddress: string): string {
  const key = cardNicknameStorageKey(stellarAddress);
  if (typeof window !== 'undefined') window.localStorage.removeItem(key);
  return DEFAULT_CARD_NICKNAME;
}

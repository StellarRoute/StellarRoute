import { afterEach, describe, expect, it } from 'vitest';

import {
  CARD_NICKNAME_STORAGE_PREFIX,
  DEFAULT_CARD_NICKNAME,
  MAX_CARD_NICKNAME_LENGTH,
  cardNicknameStorageKey,
  getCardNickname,
  loadCardNickname,
  resetCardNickname,
  sanitizeCardNickname,
  saveCardNickname,
} from './nickname';

const ADDRESS = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2';
const OTHER_ADDRESS = 'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB2';

afterEach(() => {
  window.localStorage.clear();
});

describe('card nickname storage', () => {
  it('uses a distinct localStorage key for each Stellar address', () => {
    expect(cardNicknameStorageKey(ADDRESS)).toBe(
      `${CARD_NICKNAME_STORAGE_PREFIX}:${ADDRESS}`
    );
    expect(cardNicknameStorageKey(` ${ADDRESS} `)).toBe(
      cardNicknameStorageKey(ADDRESS)
    );
    expect(cardNicknameStorageKey(ADDRESS)).not.toBe(
      cardNicknameStorageKey(OTHER_ADDRESS)
    );
    expect(() => cardNicknameStorageKey('  ')).toThrow(TypeError);
  });

  it('saves and loads a nickname only for its address', () => {
    expect(saveCardNickname(ADDRESS, 'Travel Card')).toBe('Travel Card');

    expect(loadCardNickname(ADDRESS)).toBe('Travel Card');
    expect(loadCardNickname(OTHER_ADDRESS)).toBeNull();
    expect(getCardNickname(OTHER_ADDRESS)).toBe(DEFAULT_CARD_NICKNAME);
  });

  it('strips HTML tags and trims the saved nickname', () => {
    expect(sanitizeCardNickname('  <b>Travel</b> <i>Card</i>  ')).toBe(
      'Travel Card'
    );
    expect(saveCardNickname(ADDRESS, '<b>Travel Card</b>')).toBe('Travel Card');
    expect(loadCardNickname(ADDRESS)).toBe('Travel Card');
  });

  it('limits nicknames to 24 characters when sanitizing and saving', () => {
    const longName = 'A'.repeat(MAX_CARD_NICKNAME_LENGTH + 10);

    expect(sanitizeCardNickname(longName)).toHaveLength(
      MAX_CARD_NICKNAME_LENGTH
    );
    expect(saveCardNickname(ADDRESS, longName)).toHaveLength(
      MAX_CARD_NICKNAME_LENGTH
    );
  });

  it('rejects whitespace-only names and resets an empty name to the default', () => {
    expect(() => saveCardNickname(ADDRESS, '   ')).toThrow(TypeError);
    expect(window.localStorage.length).toBe(0);

    saveCardNickname(ADDRESS, 'Travel');
    expect(saveCardNickname(ADDRESS, '')).toBe(DEFAULT_CARD_NICKNAME);
    expect(loadCardNickname(ADDRESS)).toBeNull();
    expect(getCardNickname(ADDRESS)).toBe(DEFAULT_CARD_NICKNAME);
  });

  it('resets only the selected address', () => {
    saveCardNickname(ADDRESS, 'Travel');
    saveCardNickname(OTHER_ADDRESS, 'Subscriptions');

    expect(resetCardNickname(ADDRESS)).toBe(DEFAULT_CARD_NICKNAME);
    expect(loadCardNickname(ADDRESS)).toBeNull();
    expect(loadCardNickname(OTHER_ADDRESS)).toBe('Subscriptions');
  });
});

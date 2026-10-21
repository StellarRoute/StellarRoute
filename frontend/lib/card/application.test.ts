import { afterEach, describe, expect, it, vi } from 'vitest';
import {
    CARD_APPLICATION_STORAGE_KEY,
    clearCardApplicationDraft,
    loadCardApplicationDraft,
    saveCardApplicationDraft,
    type CardApplicationDraft,
} from './application';

const draft: CardApplicationDraft = {
    name: 'Ada Lovelace',
    country: 'NG',
    preferredFiatCurrency: 'USD',
    monthlyUsdcLimit: 250,
};

afterEach(() => {
    window.localStorage.clear();
    vi.restoreAllMocks();
});

describe('card application draft', () => {
    it('saves locally and restores the draft after reload', () => {
        saveCardApplicationDraft(draft);

        expect(window.localStorage.getItem(CARD_APPLICATION_STORAGE_KEY)).toBe(JSON.stringify(draft));
        expect(loadCardApplicationDraft()).toEqual(draft);
    });

    it('rejects a zero monthly USDC limit without saving', () => {
        expect(() => saveCardApplicationDraft({ ...draft, monthlyUsdcLimit: 0 })).toThrow(TypeError);
        expect(window.localStorage.getItem(CARD_APPLICATION_STORAGE_KEY)).toBeNull();
    });

    it('does not make a network request when saving', () => {
        const fetchSpy = vi.spyOn(globalThis, 'fetch');

        saveCardApplicationDraft(draft);

        expect(fetchSpy).not.toHaveBeenCalled();
    });

    it('clears the saved draft', () => {
        saveCardApplicationDraft(draft);

        clearCardApplicationDraft();

        expect(loadCardApplicationDraft()).toBeNull();
        expect(window.localStorage.getItem(CARD_APPLICATION_STORAGE_KEY)).toBeNull();
    });

    it('rejects unsupported currencies and ignores invalid stored data', () => {
        expect(() =>
            saveCardApplicationDraft({ ...draft, preferredFiatCurrency: 'CAD' as 'USD' }),
        ).toThrow(TypeError);

        window.localStorage.setItem(
            CARD_APPLICATION_STORAGE_KEY,
            JSON.stringify({ ...draft, monthlyUsdcLimit: 0 }),
        );
        expect(loadCardApplicationDraft()).toBeNull();
    });
});
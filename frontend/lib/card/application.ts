export type CardApplicationCurrency = 'USD' | 'EUR' | 'GBP';

export interface CardApplicationDraft {
    name: string;
    country: string;
    preferredFiatCurrency: CardApplicationCurrency;
    monthlyUsdcLimit: number;
}

export const CARD_APPLICATION_STORAGE_KEY = 'stellar_route_card_application_draft';

const APPLICATION_CURRENCIES: readonly CardApplicationCurrency[] = ['USD', 'EUR', 'GBP'];

function isCardApplicationDraft(value: unknown): value is CardApplicationDraft {
    if (typeof value !== 'object' || value === null) return false;

    const draft = value as Partial<CardApplicationDraft>;
    return (
        typeof draft.name === 'string' &&
        draft.name.trim().length > 0 &&
        typeof draft.country === 'string' &&
        draft.country.trim().length > 0 &&
        APPLICATION_CURRENCIES.includes(draft.preferredFiatCurrency as CardApplicationCurrency) &&
        typeof draft.monthlyUsdcLimit === 'number' &&
        Number.isFinite(draft.monthlyUsdcLimit) &&
        draft.monthlyUsdcLimit > 0
    );
}

export function saveCardApplicationDraft(draft: CardApplicationDraft): void {
    if (!isCardApplicationDraft(draft)) {
        throw new TypeError('Card application draft must include a name, country, supported currency, and positive monthly USDC limit.');
    }
    if (typeof window === 'undefined') return;

    window.localStorage.setItem(CARD_APPLICATION_STORAGE_KEY, JSON.stringify(draft));
}

export function loadCardApplicationDraft(): CardApplicationDraft | null {
    if (typeof window === 'undefined') return null;

    try {
        const stored = window.localStorage.getItem(CARD_APPLICATION_STORAGE_KEY);
        if (!stored) return null;

        const draft: unknown = JSON.parse(stored);
        return isCardApplicationDraft(draft) ? draft : null;
    } catch {
        return null;
    }
}

export function clearCardApplicationDraft(): void {
    if (typeof window === 'undefined') return;
    window.localStorage.removeItem(CARD_APPLICATION_STORAGE_KEY);
}
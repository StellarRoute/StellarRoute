export interface Subscription {
  id: string;
  payee: string;
  asset: string;
  amount: string;
  interval: string;
  max: string;
  createdCount: number;
  nextDueDate: string;
}

const STORAGE_KEY = 'stellar_route_ai_subscriptions';

export function loadSubscriptions(): Subscription[] {
  if (typeof window === 'undefined') return [];
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return [];
    return JSON.parse(stored) as Subscription[];
  } catch (error) {
    console.error('Failed to load subscriptions from localStorage:', error);
    return [];
  }
}

export function saveSubscription(subscription: Subscription): void {
  if (typeof window === 'undefined') return;
  try {
    const subscriptions = loadSubscriptions();
    const existing = subscriptions.findIndex(s => s.id === subscription.id);
    if (existing >= 0) {
      subscriptions[existing] = subscription;
    } else {
      subscriptions.push(subscription);
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(subscriptions));
  } catch (error) {
    console.error('Failed to save subscription to localStorage:', error);
  }
}

export function cancelSubscription(id: string): void {
  if (typeof window === 'undefined') return;
  try {
    const subscriptions = loadSubscriptions();
    const filtered = subscriptions.filter(s => s.id !== id);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(filtered));
  } catch (error) {
    console.error('Failed to cancel subscription in localStorage:', error);
  }
}

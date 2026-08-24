/**
 * notificationManager — pure utility module for browser transaction notifications.
 * No React dependencies. Independently testable.
 */

export type TerminalStatus = 'confirmed' | 'failed' | 'dropped';

export interface NotificationParams {
  status: TerminalStatus;
  txHash?: string;
  fromAsset: string;
  fromAmount: string;
  toAsset: string;
  toAmount: string;
  txId: string;
}

export interface NotificationPreference {
  enabled: boolean;
}

/**
 * Returns true if the browser supports the Notification API.
 */
export function isNotificationSupported(): boolean {
  return typeof window !== 'undefined' && 'Notification' in window;
}

/**
 * Constructs the notification title for a given terminal status.
 * Returns "Swap Confirmed" | "Swap Failed" | "Swap Dropped"
 */
export function buildNotificationTitle(status: TerminalStatus): string {
  switch (status) {
    case 'confirmed':
      return 'Swap Confirmed';
    case 'failed':
      return 'Swap Failed';
    case 'dropped':
      return 'Swap Dropped';
  }
}

/**
 * Constructs the notification body string for a given terminal status.
 * - confirmed: "Swapped {fromAmount} {fromAsset} → {toAmount} {toAsset}\nTx: {txHash}"
 * - failed:    "Swap of {fromAmount} {fromAsset} → {toAmount} {toAsset} failed."
 * - dropped:   "Swap of {fromAmount} {fromAsset} → {toAmount} {toAsset} was dropped. You may resubmit."
 */
export function buildNotificationBody(params: NotificationParams): string {
  const { status, fromAmount, fromAsset, toAmount, toAsset, txHash } = params;

  switch (status) {
    case 'confirmed':
      return `Swapped ${fromAmount} ${fromAsset} → ${toAmount} ${toAsset}\nTx: ${txHash}`;
    case 'failed':
      return `Swap of ${fromAmount} ${fromAsset} → ${toAmount} ${toAsset} failed.`;
    case 'dropped':
      return `Swap of ${fromAmount} ${fromAsset} → ${toAmount} ${toAsset} was dropped. You may resubmit.`;
  }
}

/**
 * Constructs the Stellar Expert explorer URL for a transaction hash.
 * Returns "https://stellar.expert/explorer/public/tx/{txHash}"
 */
export function buildExplorerUrl(txHash: string): string {
  return `https://stellar.expert/explorer/public/tx/${txHash}`;
}

/**
 * Dispatches a browser Notification for a terminal transaction state.
 *
 * Guards — returns immediately (no-op, no throw) if:
 *   - preference.enabled is false
 *   - Notification API is unavailable
 *   - Notification.permission !== "granted"
 *
 * Does NOT mutate params.
 */
export function dispatchTransactionNotification(
  params: NotificationParams,
  preference: NotificationPreference,
): void {
  if (!preference.enabled) {
    return;
  }

  if (!isNotificationSupported()) {
    return;
  }

  if (Notification.permission !== 'granted') {
    return;
  }

  const title = buildNotificationTitle(params.status);
  const body = buildNotificationBody(params);

  const options: NotificationOptions = {
    body,
    tag: params.txId,
    icon: '/icons/icon-192.png',
    data:
      params.status === 'confirmed' && params.txHash != null
        ? { url: buildExplorerUrl(params.txHash) }
        : undefined,
  };

  try {
    new Notification(title, options);
  } catch (err) {
    console.error('[notifications] Failed to dispatch notification:', err);
  }
}

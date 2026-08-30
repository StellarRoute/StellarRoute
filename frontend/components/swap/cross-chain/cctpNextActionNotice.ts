/** User-actionable primary CTA actions that need an explicit "your turn" cue. */
export const CCTP_USER_ACTION_KEYS = [
  'quote',
  'prepare',
  'approve',
  'burn',
  'mint',
  'reattest',
  'resume',
  'reconcile_pending',
] as const;

export type CctpUserActionKey = (typeof CCTP_USER_ACTION_KEYS)[number];

export function isCctpUserAction(
  action: string,
): action is CctpUserActionKey {
  return (CCTP_USER_ACTION_KEYS as readonly string[]).includes(action);
}

export function cctpNeedsUserAction(
  action: string,
  disabled: boolean,
): boolean {
  return isCctpUserAction(action) && !disabled;
}

const NOTICE_BY_ACTION: Record<CctpUserActionKey, string> = {
  quote: 'Ready when you are — get a quote to start.',
  prepare: 'Your turn — prepare the source transaction.',
  approve: 'Your turn — approve USDC spending in your wallet.',
  burn: 'Your turn — confirm the lock on the source chain.',
  mint: 'Your turn — confirm receive on the destination chain.',
  reattest: 'Your turn — retry confirmation.',
  resume: 'Your turn — resume the saved transfer.',
  reconcile_pending: 'Your turn — reconcile the pending transaction.',
};

export function resolveCctpNextActionNotice(action: string): string | null {
  if (!isCctpUserAction(action)) return null;
  return NOTICE_BY_ACTION[action];
}

/** Toast title when transitioning into a clickable next step. */
export function resolveCctpNextActionToast(action: string): string | null {
  if (!isCctpUserAction(action)) return null;
  switch (action) {
    case 'approve':
      return 'Next: approve USDC in your wallet';
    case 'burn':
      return 'Next: confirm lock on source chain';
    case 'mint':
      return 'Next: confirm receive on destination';
    case 'prepare':
      return 'Next: prepare source transaction';
    case 'reattest':
      return 'Next: retry confirmation';
    case 'resume':
      return 'Next: resume transfer';
    case 'reconcile_pending':
      return 'Next: reconcile pending transaction';
    case 'quote':
      return null;
    default:
      return null;
  }
}

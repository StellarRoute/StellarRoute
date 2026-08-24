'use client';

import { Bell, BellOff } from 'lucide-react';

import { cn } from '@/lib/utils';
import { useSwapI18n } from '@/lib/swap-i18n';

export interface BrowserNotificationSettingsProps {
  browserNotifications: boolean;
  permissionState: NotificationPermission | 'unsupported';
  isDisabled: boolean;
  onEnable: () => Promise<void>;
  onDisable: () => void;
}

export function BrowserNotificationSettings({
  browserNotifications,
  permissionState,
  isDisabled,
  onEnable,
  onDisable,
}: BrowserNotificationSettingsProps) {
  const { t } = useSwapI18n();
  const handleToggle = () => {
    if (isDisabled) return;
    if (browserNotifications) {
      onDisable();
    } else {
      void onEnable();
    }
  };

  // Derive accessible label based on state
  let ariaLabel: string;
  if (isDisabled && permissionState === 'denied') {
    ariaLabel = t('settings.notifications.blockedAria');
  } else if (isDisabled && permissionState === 'unsupported') {
    ariaLabel = t('settings.notifications.unsupportedAria');
  } else if (browserNotifications) {
    ariaLabel = t('settings.notifications.enabledAria');
  } else {
    ariaLabel = t('settings.notifications.disabledAria');
  }

  const Icon = browserNotifications && !isDisabled ? Bell : BellOff;

  return (
    <div className="space-y-1 pt-4 border-t border-border/20">
      <div className="flex items-center justify-between min-h-[44px]">
        <div className="flex items-center gap-2">
          <Icon
            className={cn(
              'h-4 w-4 transition-colors duration-300',
              browserNotifications && !isDisabled
                ? 'text-primary'
                : 'text-muted-foreground',
            )}
          />
          <span className="text-sm font-semibold tracking-tight">
            {t('settings.notifications.transactionLabel')}
          </span>
        </div>

        <button
          role="switch"
          aria-checked={browserNotifications}
          aria-label={ariaLabel}
          disabled={isDisabled}
          onClick={handleToggle}
          className={cn(
            'relative inline-flex h-6 w-11 shrink-0 rounded-full border-2 border-transparent transition-colors duration-300',
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background',
            'min-h-[44px] min-w-[44px] items-center',
            isDisabled
              ? 'cursor-not-allowed opacity-40 bg-muted'
              : browserNotifications
                ? 'cursor-pointer bg-primary shadow-lg shadow-primary/20'
                : 'cursor-pointer bg-muted',
          )}
        >
          <span
            className={cn(
              'pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform duration-300',
              browserNotifications && !isDisabled
                ? 'translate-x-5'
                : 'translate-x-0',
            )}
          />
        </button>
      </div>

      {/* Contextual hint when disabled */}
      {isDisabled && (
        <p className="text-[10px] text-muted-foreground leading-normal pl-6">
          {permissionState === 'denied'
            ? t('settings.notifications.blocked')
            : t('settings.notifications.unsupported')}
        </p>
      )}
    </div>
  );
}

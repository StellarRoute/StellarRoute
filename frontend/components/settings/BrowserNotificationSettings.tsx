'use client';

import { Bell, BellOff } from 'lucide-react';

import { cn } from '@/lib/utils';

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
    ariaLabel =
      'Browser notifications: blocked by browser. Change this in your browser settings.';
  } else if (isDisabled && permissionState === 'unsupported') {
    ariaLabel =
      'Browser notifications: not supported in this browser.';
  } else if (browserNotifications) {
    ariaLabel = 'Browser notifications: enabled. Click to disable.';
  } else {
    ariaLabel = 'Browser notifications: disabled. Click to enable.';
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
            Transaction Notifications
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
            ? 'Notifications are blocked by your browser. Enable them in your browser settings to use this feature.'
            : 'Your browser does not support desktop notifications.'}
        </p>
      )}
    </div>
  );
}

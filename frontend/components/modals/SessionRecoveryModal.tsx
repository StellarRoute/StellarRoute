import React, { useEffect, useState } from 'react';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { AlertCircle, RefreshCw } from 'lucide-react';

interface SessionRecoveryModalProps {
  isOpen: boolean;
  isRecovering: boolean;
  refreshType: 'sleep' | 'refresh' | null;
  onRestore: () => Promise<void>;
  onDismiss: () => void;
}

export function SessionRecoveryModal({
  isOpen,
  isRecovering,
  refreshType,
  onRestore,
  onDismiss,
}: SessionRecoveryModalProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setIsLoading(isRecovering);
    setError(null);
  }, [isRecovering]);

  const handleRestore = async () => {
    setIsLoading(true);
    setError(null);
    try {
      await onRestore();
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Failed to restore session';
      setError(message);
      setIsLoading(false);
    }
  };

  const getTitle = () => {
    switch (refreshType) {
      case 'sleep':
        return 'Tab Recovered from Sleep';
      case 'refresh':
        return 'Page Refreshed';
      default:
        return 'Session Recovery';
    }
  };

  const getDescription = () => {
    switch (refreshType) {
      case 'sleep':
        return 'Your browser tab was inactive. Would you like to restore your previous trading session?';
      case 'refresh':
        return 'The page was refreshed. Would you like to restore your previous trading session?';
      default:
        return 'Restore your previous trading session?';
    }
  };

  return (
    <AlertDialog open={isOpen}>
      <AlertDialogContent>
        <div className="flex gap-3">
          <AlertCircle className="h-5 w-5 text-warning flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <AlertDialogHeader>
              <AlertDialogTitle>{getTitle()}</AlertDialogTitle>
              <AlertDialogDescription>
                {getDescription()}
              </AlertDialogDescription>
            </AlertDialogHeader>

            {error && (
              <div className="my-3 p-3 bg-destructive/10 border border-destructive/20 rounded-md text-sm text-destructive">
                {error}
              </div>
            )}

            <div className="mt-6 flex gap-3 justify-end">
              <AlertDialogCancel disabled={isLoading} onClick={onDismiss}>
                Start Fresh
              </AlertDialogCancel>
              <AlertDialogAction
                disabled={isLoading}
                onClick={handleRestore}
                className="gap-2"
              >
                {isLoading && <RefreshCw className="h-4 w-4 animate-spin" />}
                {isLoading ? 'Restoring...' : 'Restore Session'}
              </AlertDialogAction>
            </div>
          </div>
        </div>
      </AlertDialogContent>
    </AlertDialog>
  );
}

'use client';

import { useMemo, useState } from 'react';
import { Share2, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ExplorerLink } from '@/components/shared/ExplorerLink';
import { CopyButton } from '@/components/shared/CopyButton';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';

export interface PostSwapSuccessScreenProps {
  txHash: string;
  explorerUrl?: string;
  className?: string;
}

export function PostSwapSuccessScreen({
  txHash,
  explorerUrl,
  className,
}: PostSwapSuccessScreenProps) {
  const url = useMemo(
    () =>
      explorerUrl ??
      `https://stellar.expert/explorer/public/tx/${encodeURIComponent(txHash)}`,
    [explorerUrl, txHash]
  );

  const [isSharing, setIsSharing] = useState(false);
  const [copiedExplorer, setCopiedExplorer] = useState(false);

  const handleShare = async () => {
    if (!txHash) return;
    setIsSharing(true);
    try {
      const canShare =
        typeof navigator !== 'undefined' &&
        typeof navigator.share === 'function' &&
        navigator.canShare?.({ url });

      if (
        typeof navigator !== 'undefined' &&
        typeof navigator.share === 'function' &&
        canShare !== false
      ) {
        await navigator.share({
          title: 'StellarRoute Swap',
          text: 'View this confirmed swap on Stellar Explorer',
          url,
        });
        return;
      }

      await navigator.clipboard.writeText(url);
      setCopiedExplorer(true);
      toast.success('Explorer link copied');
      window.setTimeout(() => setCopiedExplorer(false), 2000);
    } catch (err) {
      if ((err as Error)?.name !== 'AbortError') {
        toast.error('Failed to share');
      }
    } finally {
      setIsSharing(false);
    }
  };

  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center gap-6 text-center',
        className
      )}
    >
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-widest text-success">
          Swap confirmed
        </p>
        <h2 className="text-2xl font-bold">Your swap is complete</h2>
        <p className="text-sm text-muted-foreground">
          View the transaction in an explorer or share the link.
        </p>
      </div>

      <div className="w-full space-y-4">
        <div className="min-h-[44px] flex flex-col items-center gap-2">
          <div className="flex items-center gap-2">
            <span className="font-mono text-xs text-muted-foreground truncate max-w-[260px]">
              {txHash}
            </span>
            <CopyButton value={txHash} label="Copy transaction hash" />
          </div>
          <ExplorerLink
            hash={txHash}
            className="flex items-center gap-1 text-sm text-primary hover:underline"
          />
        </div>

        <div className="flex flex-col sm:flex-row gap-3 sm:items-center sm:justify-center">
          <Button
            type="button"
            onClick={handleShare}
            disabled={isSharing}
            className="flex-1 h-11 rounded-xl font-bold shadow-lg shadow-green-500/20"
            aria-label="Share explorer link"
          >
            {isSharing ? (
              'Sharing…'
            ) : copiedExplorer ? (
              <span className="flex items-center gap-2">
                <Check className="h-4 w-4" aria-hidden /> Copied
              </span>
            ) : (
              <span className="flex items-center gap-2">
                <Share2 className="h-4 w-4" aria-hidden /> Share
              </span>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}

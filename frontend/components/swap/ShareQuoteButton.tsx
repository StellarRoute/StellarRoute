'use client';

import { useState } from 'react';
import { Share2, Check, Copy } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Input } from '@/components/ui/input';
import { useShareableQuote } from '@/hooks/useShareableQuote';
import type { ShareableQuoteParams } from '@/hooks/useShareableQuote';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';

interface ShareQuoteButtonProps {
  params: ShareableQuoteParams;
  disabled?: boolean;
  className?: string;
}

/**
 * Button to generate and share a read-only quote link
 * Encodes pair, amount, and slippage in URL params
 */
export function ShareQuoteButton({
  params,
  disabled = false,
  className,
}: ShareQuoteButtonProps) {
  const { generateShareableUrl } = useShareableQuote();
  const [isOpen, setIsOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const [shareUrl, setShareUrl] = useState<string | null>(null);

  const handleOpen = () => {
    const url = generateShareableUrl(params);
    if (url) {
      setShareUrl(url);
      setIsOpen(true);
    } else {
      toast.error('Failed to generate shareable link');
    }
  };

  const handleCopy = async () => {
    if (!shareUrl) return;

    try {
      await navigator.clipboard.writeText(shareUrl);
      setCopied(true);
      toast.success('Link copied to clipboard');
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      toast.error('Failed to copy link');
    }
  };

  const handleShare = async () => {
    if (!shareUrl) return;

    if (navigator.share) {
      try {
        await navigator.share({
          title: 'StellarRoute Quote',
          text: 'Check out this swap quote on StellarRoute',
          url: shareUrl,
        });
      } catch (error) {
        // User cancelled or share failed
        if ((error as Error).name !== 'AbortError') {
          toast.error('Failed to share link');
        }
      }
    } else {
      await handleCopy();
    }
  };

  return (
    <Popover open={isOpen} onOpenChange={setIsOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleOpen}
          disabled={disabled}
          className={cn('h-8 gap-1.5 text-xs', className)}
          aria-label="Share quote"
        >
          <Share2 className="h-3.5 w-3.5" />
          Share
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-80" align="end">
        <div className="space-y-3">
          <div>
            <h4 className="font-semibold text-sm mb-1">Share Quote</h4>
            <p className="text-xs text-muted-foreground">
              Share this read-only quote link with others
            </p>
          </div>
          <div className="flex gap-2">
            <Input
              value={shareUrl || ''}
              readOnly
              className="text-xs font-mono"
              onClick={(e) => e.currentTarget.select()}
            />
            <Button
              size="sm"
              variant="outline"
              onClick={handleCopy}
              className="shrink-0"
              aria-label={copied ? 'Copied' : 'Copy link'}
            >
              {copied ? (
                <Check className="h-4 w-4 text-green-600" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </Button>
          </div>
          {typeof navigator !== 'undefined' && typeof navigator.share === 'function' && (
            <Button
              size="sm"
              variant="default"
              onClick={handleShare}
              className="w-full"
            >
              <Share2 className="h-3.5 w-3.5 mr-1.5" />
              Share via...
            </Button>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}

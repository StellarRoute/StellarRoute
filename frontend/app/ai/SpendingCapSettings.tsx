'use client';

import * as React from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { saveCap } from '@/lib/ai/cap';

export interface SpendingCapSettingsProps {
  cap: number | null;
  onChange: (cap: number | null) => void;
}

/** /ai settings: local per-confirm USDC cap (AI-31). Not a custody control. */
export function SpendingCapSettings({ cap, onChange }: SpendingCapSettingsProps) {
  const [draft, setDraft] = React.useState(cap === null ? '' : String(cap));

  React.useEffect(() => {
    setDraft(cap === null ? '' : String(cap));
  }, [cap]);

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    onChange(saveCap(draft));
  };

  const handleClear = () => {
    setDraft('');
    onChange(saveCap(''));
  };

  return (
    <section data-testid="ai-settings" className="rounded-lg border border-border p-4 space-y-3">
      <div>
        <h2 className="text-sm font-semibold">Spending cap</h2>
        <p className="mt-1 text-xs text-muted-foreground">
          {cap === null
            ? 'No cap set. Confirm is not limited by amount.'
            : `Confirm is disabled above ${cap} USDC. Stored only in this browser.`}
        </p>
      </div>
      <form onSubmit={handleSave} className="flex gap-2">
        <Input
          type="text"
          inputMode="decimal"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="Max USDC per confirm"
          aria-label="Max USDC per confirm"
          className="flex-1"
          data-testid="cap-input"
        />
        <Button type="submit" size="sm" variant="outline" data-testid="cap-save">
          Save
        </Button>
        {cap !== null && (
          <Button type="button" size="sm" variant="ghost" onClick={handleClear} data-testid="cap-clear">
            Clear
          </Button>
        )}
      </form>
    </section>
  );
}

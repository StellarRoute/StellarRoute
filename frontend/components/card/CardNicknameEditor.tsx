'use client';

import { useEffect, useId, useState } from 'react';
import { Check, RotateCcw, X } from 'lucide-react';

import {
  DEFAULT_CARD_NICKNAME,
  getCardNickname,
  MAX_CARD_NICKNAME_LENGTH,
  resetCardNickname,
  sanitizeCardNickname,
  saveCardNickname,
} from '@/lib/card/nickname';

export interface CardNicknameEditorProps {
  /** Active Stellar public key; the nickname is stored only for this address. */
  stellarAddress: string | null;
  className?: string;
}

export function CardNicknameEditor({
  stellarAddress,
  className = '',
}: CardNicknameEditorProps) {
  const [storedNickname, setStoredNickname] = useState<{
    address: string | null;
    nickname: string;
  }>({ address: null, nickname: DEFAULT_CARD_NICKNAME });
  const [draft, setDraft] = useState('');
  const [isEditing, setIsEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputId = `card-nickname-${useId().replace(/:/g, '')}`;
  const nickname =
    storedNickname.address === stellarAddress
      ? storedNickname.nickname
      : DEFAULT_CARD_NICKNAME;

  useEffect(() => {
    setStoredNickname({
      address: stellarAddress,
      nickname: stellarAddress
        ? getCardNickname(stellarAddress)
        : DEFAULT_CARD_NICKNAME,
    });
    setDraft('');
    setIsEditing(false);
    setError(null);
  }, [stellarAddress]);

  const handleEdit = () => {
    setDraft(nickname === DEFAULT_CARD_NICKNAME ? '' : nickname);
    setError(null);
    setIsEditing(true);
  };

  const handleSave = () => {
    if (!stellarAddress) return;

    if (draft.length === 0) {
      setStoredNickname({
        address: stellarAddress,
        nickname: saveCardNickname(stellarAddress, ''),
      });
      setIsEditing(false);
      setError(null);
      return;
    }

    const sanitized = sanitizeCardNickname(draft);
    if (sanitized.length === 0) {
      setError(
        'Enter a nickname, or leave the field empty to use the default.'
      );
      return;
    }

    setStoredNickname({
      address: stellarAddress,
      nickname: saveCardNickname(stellarAddress, sanitized),
    });
    setIsEditing(false);
    setError(null);
  };

  const handleReset = () => {
    if (!stellarAddress) return;
    setStoredNickname({
      address: stellarAddress,
      nickname: resetCardNickname(stellarAddress),
    });
    setDraft('');
    setError(null);
    setIsEditing(false);
  };

  const handleCancel = () => {
    setDraft('');
    setError(null);
    setIsEditing(false);
  };

  return (
    <div className={`flex flex-wrap items-center gap-2 ${className}`}>
      {isEditing ? (
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
          <label className="sr-only" htmlFor={inputId}>
            Card nickname
          </label>
          <input
            id={inputId}
            type="text"
            value={draft}
            maxLength={MAX_CARD_NICKNAME_LENGTH}
            placeholder={DEFAULT_CARD_NICKNAME}
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? `${inputId}-error` : undefined}
            onChange={(event) => {
              setDraft(event.target.value);
              setError(null);
            }}
            className="min-w-0 rounded-lg border border-border bg-background px-3 py-2 text-base font-semibold text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
            data-testid="card-nickname-input"
          />
          <span className="text-xs text-muted-foreground" aria-live="polite">
            {draft.length}/{MAX_CARD_NICKNAME_LENGTH}
          </span>
          <button
            type="button"
            onClick={handleSave}
            className="inline-flex items-center gap-1 rounded-lg px-2 py-2 text-sm font-medium text-foreground hover:bg-muted"
            aria-label="Save card nickname"
            data-testid="card-nickname-save"
          >
            <Check className="h-4 w-4" aria-hidden="true" />
            Save
          </button>
          <button
            type="button"
            onClick={handleReset}
            disabled={!stellarAddress}
            className="inline-flex items-center gap-1 rounded-lg px-2 py-2 text-sm text-muted-foreground hover:bg-muted disabled:opacity-50"
            aria-label="Reset card nickname to default"
            data-testid="card-nickname-reset"
          >
            <RotateCcw className="h-4 w-4" aria-hidden="true" />
            Reset
          </button>
          <button
            type="button"
            onClick={handleCancel}
            className="rounded-lg p-2 text-muted-foreground hover:bg-muted"
            aria-label="Cancel editing card nickname"
            data-testid="card-nickname-cancel"
          >
            <X className="h-4 w-4" aria-hidden="true" />
          </button>
          {error && (
            <p
              id={`${inputId}-error`}
              role="alert"
              className="w-full text-xs font-medium text-destructive"
              data-testid="card-nickname-error"
            >
              {error}
            </p>
          )}
        </div>
      ) : (
        <>
          <h2
            className="min-w-0 truncate text-lg font-semibold text-foreground"
            data-testid="card-nickname-title"
          >
            {nickname}
          </h2>
          <button
            type="button"
            onClick={handleEdit}
            disabled={!stellarAddress}
            className="rounded-lg p-2 text-muted-foreground transition hover:bg-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
            aria-label="Edit card nickname"
            data-testid="card-nickname-edit"
          >
            <svg
              viewBox="0 0 24 24"
              className="h-4 w-4"
              fill="none"
              stroke="currentColor"
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="m16 4 4 4L9 19l-5 1 1-5L16 4Z" />
              <path d="m14 6 4 4" />
            </svg>
          </button>
        </>
      )}
    </div>
  );
}

export default CardNicknameEditor;

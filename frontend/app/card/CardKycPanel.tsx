"use client";

import { useCallback, useEffect, useState } from "react";

/**
 * Local-only record that the user ticked the KYC acknowledgement.
 *
 * Nothing about the applicant is stored — just the fact that the box was
 * ticked. No card number, document reference, or upload ever reaches this key.
 */
export const CARD_KYC_ACK_KEY = "stellarroute:card:kyc-acknowledged";

/**
 * Compliance doc the panel links to.
 *
 * Kept as a single overridable constant because the doc itself lands with
 * CARD-05; the panel never hardcodes the path inline, so there is exactly one
 * place to repoint it.
 */
export const CARD_COMPLIANCE_DOC_PATH =
  "https://github.com/StellarRoute/StellarRoute/blob/main/docs/card/compliance.md";

/** Statuses the card application can be in. Unknown values are tolerated. */
export type CardApplicationStatus = "kyc_required" | (string & {});

export interface CardKycPanelProps {
  /** Application status. The panel renders only for `kyc_required`. */
  status: CardApplicationStatus | null | undefined;
  /** Override for the compliance doc link. */
  complianceDocHref?: string;
}

/**
 * KYC gate shown before any card application (CARD-06).
 *
 * An active card cannot be previewed as if KYC were optional, so while the
 * status is `kyc_required` the continue control stays disabled until the user
 * ticks an acknowledgement that is stored locally.
 *
 * Deliberately no file input and no document upload: StellarRoute never holds
 * card credentials, so there is nothing here to attach.
 */
export function CardKycPanel({
  status,
  complianceDocHref = CARD_COMPLIANCE_DOC_PATH,
}: CardKycPanelProps) {
  const [acknowledged, setAcknowledged] = useState(false);

  // Read after mount so server and first client render agree.
  useEffect(() => {
    try {
      setAcknowledged(window.localStorage.getItem(CARD_KYC_ACK_KEY) === "1");
    } catch {
      setAcknowledged(false);
    }
  }, []);

  const toggleAcknowledgement = useCallback(() => {
    setAcknowledged((previous) => {
      const next = !previous;
      try {
        if (next) {
          window.localStorage.setItem(CARD_KYC_ACK_KEY, "1");
        } else {
          window.localStorage.removeItem(CARD_KYC_ACK_KEY);
        }
      } catch {
        // Storage is best-effort: if it is unavailable the box still works for
        // this session, it just will not persist.
      }
      return next;
    });
  }, []);

  // Both hooks above run unconditionally, before this early return, so hook
  // order stays stable as the status changes.
  if (status !== "kyc_required") return null;

  return (
    <section
      aria-labelledby="card-kyc-heading"
      data-testid="card-kyc-panel"
      className="rounded-xl border border-amber-200 bg-amber-50 p-4 dark:border-amber-900 dark:bg-amber-950/30"
    >
      <h2
        id="card-kyc-heading"
        className="text-sm font-semibold text-amber-900 dark:text-amber-100"
      >
        Identity verification is required first
      </h2>
      <p className="mt-1 text-sm text-amber-800 dark:text-amber-200">
        Before you can apply, the card partner needs to verify your identity.
        Read the compliance notes, then confirm you have understood them.
      </p>
      <p className="mt-2 text-sm">
        <a
          href={complianceDocHref}
          target="_blank"
          rel="noreferrer noopener"
          data-testid="card-kyc-doc-link"
          className="font-medium text-amber-900 underline underline-offset-2 dark:text-amber-100"
        >
          Read the compliance notes
        </a>
      </p>

      <label className="mt-3 flex items-start gap-2 text-sm text-amber-900 dark:text-amber-100">
        <input
          type="checkbox"
          checked={acknowledged}
          onChange={toggleAcknowledgement}
          data-testid="card-kyc-ack"
          className="mt-0.5"
        />
        <span>
          I have read the compliance notes and understand that verification is
          required before applying.
        </span>
      </label>

      <button
        type="button"
        disabled={!acknowledged}
        data-testid="card-kyc-continue"
        className="mt-3 rounded-md bg-amber-900 px-3 py-1.5 text-sm font-medium text-amber-50 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-amber-100 dark:text-amber-950"
      >
        Continue
      </button>
    </section>
  );
}

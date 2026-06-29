"use client";

import { useRef } from "react";
import { X, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  ONBOARDING_STEPS,
  useOnboardingChecklist,
  type OnboardingStepId,
} from "@/hooks/useOnboardingChecklist";

interface OnboardingChecklistProps {
  /** Externally completed steps (e.g. wallet connected) */
  completedSteps?: OnboardingStepId[];
}

export function OnboardingChecklist({ completedSteps = [] }: OnboardingChecklistProps) {
  const { dismissed, dismiss } = useOnboardingChecklist();
  const skipRef = useRef<HTMLButtonElement>(null);

  if (dismissed) return null;

  const allCompleted = new Set(completedSteps);

  function handleStepClick(anchor?: string) {
    if (!anchor) return;
    const el = document.querySelector(anchor);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      if (el instanceof HTMLElement) el.focus();
    }
  }

  return (
    <section
      aria-label="Getting started checklist"
      aria-live="polite"
      className="rounded-xl border bg-card p-4 shadow-sm"
    >
      <div className="flex items-center justify-between mb-3">
        <h2 className="text-sm font-semibold">Get started with StellarRoute</h2>
        <Button
          ref={skipRef}
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={dismiss}
          aria-label="Dismiss onboarding checklist"
        >
          <X className="h-4 w-4" aria-hidden="true" />
        </Button>
      </div>

      <ol className="space-y-2" role="list">
        {ONBOARDING_STEPS.map((step, idx) => {
          const done = allCompleted.has(step.id);
          return (
            <li key={step.id} className="flex items-start gap-3">
              <span
                aria-hidden="true"
                className={`mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border text-xs font-bold
                  ${done ? "border-green-500 bg-green-500 text-white" : "border-muted-foreground text-muted-foreground"}`}
              >
                {done ? <Check className="h-3 w-3" /> : idx + 1}
              </span>
              <div className="min-w-0 flex-1">
                <button
                  className="text-left text-sm font-medium hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  onClick={() => handleStepClick(step.anchor)}
                  aria-label={`${step.label}${done ? " (completed)" : ""}`}
                >
                  {step.label}
                </button>
                <p className="text-xs text-muted-foreground">{step.description}</p>
              </div>
              {done && (
                <span className="sr-only">Completed</span>
              )}
            </li>
          );
        })}
      </ol>

      <Button
        variant="ghost"
        size="sm"
        className="mt-3 w-full text-xs text-muted-foreground"
        onClick={dismiss}
      >
        Skip — I know what I&apos;m doing
      </Button>
    </section>
  );
}

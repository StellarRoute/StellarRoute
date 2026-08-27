'use client';

import Link from "next/link";
import { ExternalLink } from "lucide-react";
import { useGuideI18n, GuideTranslationKey } from "@/lib/guide-i18n";

export function GuidePageClient() {
  const { t } = useGuideI18n();

  const steps: Array<{
    titleKey: GuideTranslationKey;
    bodyKey: GuideTranslationKey;
  }> = [
    { titleKey: "guide.step1.title", bodyKey: "guide.step1.body" },
    { titleKey: "guide.step2.title", bodyKey: "guide.step2.body" },
    { titleKey: "guide.step3.title", bodyKey: "guide.step3.body" },
    { titleKey: "guide.step4.title", bodyKey: "guide.step4.body" },
    { titleKey: "guide.step5.title", bodyKey: "guide.step5.body" },
    { titleKey: "guide.step6.title", bodyKey: "guide.step6.body" },
  ];

  return (
    <main className="min-h-[calc(100vh-80px)] px-4 py-10 sm:px-6 lg:px-8">
      <div className="container mx-auto max-w-3xl space-y-8">
        <header className="space-y-3">
          <p className="text-sm font-medium uppercase text-muted-foreground">
            {t('guide.header.eyebrow')}
          </p>
          <h1 className="text-3xl font-extrabold tracking-tight sm:text-4xl">
            {t('guide.header.title')}
          </h1>
          <p className="max-w-2xl text-lg text-muted-foreground">
            {t('guide.header.description')}
          </p>
          <div className="flex flex-wrap gap-3 pt-1">
            <Link
              href="/swap"
              className="inline-flex h-10 items-center rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              {t('guide.cta.openSwap')}
            </Link>
            <a
              href="https://github.com/StellarRoute/StellarRoute/blob/main/docs/user-guide-first-live-swap.md"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex h-10 items-center gap-1.5 rounded-md border px-4 text-sm font-medium hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              {t('guide.cta.fullGuide')}
              <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
            </a>
          </div>
        </header>

        <ol className="space-y-4">
          {steps.map((step, index) => (
            <li
              key={step.titleKey}
              className="rounded-xl border bg-card p-5 text-card-foreground"
            >
              <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {t('guide.step.label', { number: index + 1 })}
              </p>
              <h2 className="mt-1 text-lg font-semibold">{t(step.titleKey)}</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                {t(step.bodyKey)}
              </p>
            </li>
          ))}
        </ol>

        <aside className="rounded-xl border border-dashed p-5 text-sm text-muted-foreground">
          <p className="font-medium text-foreground">{t('guide.aside.title')}</p>
          <p className="mt-2 leading-6">
            {t('guide.aside.bodyPrefix')}{" "}
            <a
              href="https://github.com/StellarRoute/StellarRoute/blob/main/docs/risk-disclosure.md"
              target="_blank"
              rel="noopener noreferrer"
              className="font-medium text-foreground underline-offset-4 hover:underline"
            >
              {t('guide.aside.riskDisclosure')}
            </a>{" "}
            {t('guide.aside.bodySuffix')}
          </p>
        </aside>
      </div>
    </main>
  );
}

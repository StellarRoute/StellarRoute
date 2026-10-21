'use client';

import { useFeatureFlag } from '@/hooks/useFeatureFlag';

export default function AgentGuideSection() {
  const { enabled } = useFeatureFlag('ai_agent');

  if (!enabled) {
    return null;
  }

  return (
    <section
      data-testid="guide-agent-section"
      className="rounded-xl border bg-card p-5 text-card-foreground"
    >
      <h2 className="text-lg font-semibold">Agent assistance (preview)</h2>
      <p className="mt-2 text-sm leading-6 text-muted-foreground">
        When the AI agent preview is enabled, it can draft a swap for you, but
        it uses confirm-before-sign: review amounts in your wallet prompt and
        sign there. The agent never auto-signs and StellarRoute never holds
        your keys.
      </p>
    </section>
  );
}

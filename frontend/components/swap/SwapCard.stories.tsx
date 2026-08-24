import type { Story } from '@ladle/react';
import '@/app/globals.css';
import { SwapCard } from './SwapCard';
import { SwapCardStoryProviders } from './SwapCardStoryProviders';
import type { SwapCardStoryFixture } from './swapCardStory';

function SwapCardStory({
  fixture,
}: {
  fixture: SwapCardStoryFixture;
}) {
  return (
    <SwapCardStoryProviders fixture={fixture}>
      <SwapCard storyFixture={fixture} />
    </SwapCardStoryProviders>
  );
}

/** Disconnected wallet, empty form — default landing state. */
export const Idle: Story = () => <SwapCardStory fixture="idle" />;
Idle.storyName = 'Idle — No Wallet';

/** Connected wallet with an in-flight quote request. */
export const Quoting: Story = () => <SwapCardStory fixture="quoting" />;
Quoting.storyName = 'Quoting — Loading Quote';

/** Quote succeeded earlier but is now marked stale. */
export const Stale: Story = () => <SwapCardStory fixture="stale" />;
Stale.storyName = 'Stale — Outdated Quote';

/** Review step before wallet signature. */
export const Confirming: Story = () => <SwapCardStory fixture="confirming" />;
Confirming.storyName = 'Confirming — Review Modal';

/** Quote API failure with actionable error copy. */
export const Error: Story = () => <SwapCardStory fixture="error" />;
Error.storyName = 'Error — Quote Unavailable';

/** Full state matrix at a glance for design review. */
export const StateMatrix: Story = () => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="grid gap-10 max-w-5xl mx-auto">
      {(
        [
          ['idle', 'Idle'],
          ['quoting', 'Quoting'],
          ['stale', 'Stale'],
          ['confirming', 'Confirming'],
          ['error', 'Error'],
        ] as const
      ).map(([fixture, label]) => (
        <section key={fixture} className="space-y-3">
          <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
            {label}
          </h3>
          <SwapCardStoryProviders fixture={fixture}>
            <SwapCard storyFixture={fixture} />
          </SwapCardStoryProviders>
        </section>
      ))}
    </div>
  </div>
);
StateMatrix.storyName = 'State Matrix — All Fixtures';

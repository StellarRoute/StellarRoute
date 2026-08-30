import type { Story } from '@ladle/react';
import '@/app/globals.css';
import { OfframpDashboard } from './OfframpDashboard';

/** Default landing state — empty form, no amount entered, direct mode selected. */
export const Default: Story = () => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="mx-auto max-w-5xl">
      <OfframpDashboard />
    </div>
  </div>
);
Default.storyName = 'Dashboard — Empty (Default)';

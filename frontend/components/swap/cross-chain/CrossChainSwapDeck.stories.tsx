import type { Story } from '@ladle/react';
import '@/app/globals.css';
import { CrossChainSwapDeck } from './CrossChainSwapDeck';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { WalletProvider } from '@/components/providers/wallet-provider';
import { ThemeProvider } from 'next-themes';
import type { CrossChainDeckStoryPresentation } from './crossChainStoryPresentation';

function DeckStory({ presentation }: { presentation?: CrossChainDeckStoryPresentation }) {
  return (
    <ThemeProvider attribute="class" defaultTheme="dark" enableSystem>
      <SettingsProvider>
        <WalletProvider>
          <div className="dark min-h-screen bg-background p-6 text-foreground">
            <CrossChainSwapDeck storyPresentation={presentation} />
          </div>
        </WalletProvider>
      </SettingsProvider>
    </ThemeProvider>
  );
}

export const StellarNative: Story = () => <DeckStory />;
StellarNative.storyName = 'Stellar native delegates SwapCard';

export const EvmToStellar: Story = () => (
  <DeckStory
    presentation={{
      initialSourceChainId: 'ethereum-sepolia',
      initialDestChainId: 'stellar',
    }}
  />
);
EvmToStellar.storyName = 'EVM Sepolia to Stellar';

export const WalletsPartial: Story = () => (
  <DeckStory
    presentation={{
      initialSourceChainId: 'ethereum-sepolia',
      initialDestChainId: 'stellar',
      sourceWalletState: 'connected',
      destWalletState: 'disconnected',
    }}
  />
);
WalletsPartial.storyName = 'Wallets partial connect';

export const NetworkMismatch: Story = () => (
  <DeckStory
    presentation={{
      sourceWalletState: 'mismatch',
      destWalletState: 'mismatch',
    }}
  />
);
NetworkMismatch.storyName = 'Network mismatch';

export const SimplifiedBridge: Story = () => (
  <DeckStory
    presentation={{
      initialSourceChainId: 'ethereum-sepolia',
      initialDestChainId: 'stellar',
    }}
  />
);
SimplifiedBridge.storyName = 'Simplified single-column bridge';

export const UnsupportedCorridor: Story = () => (
  <DeckStory
    presentation={{
      initialSourceChainId: 'solana',
      initialDestChainId: 'stellar',
    }}
  />
);
UnsupportedCorridor.storyName = 'Unsupported corridor alert';

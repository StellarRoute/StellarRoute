'use client';

import { ReactNode } from 'react';
import { ThemeProvider as NextThemesProvider } from 'next-themes';

import { WalletProvider } from '@/components/providers/wallet-provider';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { SessionRecoveryProvider } from '@/components/providers/session-recovery-provider';

interface ProvidersProps {
  children: ReactNode;
  defaultTheme?: string;
}

export function Providers({ children, defaultTheme = 'dark' }: ProvidersProps) {
  return (
    <NextThemesProvider
      attribute="class"
      defaultTheme={defaultTheme}
      enableSystem
      disableTransitionOnChange
    >
      <SessionRecoveryProvider>
        <SettingsProvider>
          <WalletProvider defaultNetwork="testnet">{children}</WalletProvider>
        </SettingsProvider>
      </SessionRecoveryProvider>
    </NextThemesProvider>
  );
}

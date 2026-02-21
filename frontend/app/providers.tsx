'use client';

import { ReactNode } from 'react';
import { ThemeProvider } from '@/components/theme-provider';

interface ProvidersProps {
  children: ReactNode;
}

export function Providers({ children }: ProvidersProps) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="dark"
      enableSystem={false}
      storageKey="stellar-theme"
      disableTransitionOnChange
    >
      {children}
    </ThemeProvider>
  );
}

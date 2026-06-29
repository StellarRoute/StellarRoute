import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'Settings | StellarRoute',
  description:
    'Customize your StellarRoute experience with theme, language, and notification settings.',
  openGraph: {
    title: 'Settings | StellarRoute',
    description: 'Personalize your StellarRoute interface and preferences.',
    type: 'website',
  },
};

export default function SettingsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}

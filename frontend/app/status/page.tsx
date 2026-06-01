import { Metadata } from 'next';
import { StatusDashboard } from '@/components/status/StatusDashboard';

export const metadata: Metadata = {
  title: 'API Status | StellarRoute',
  description: 'Live health status of StellarRoute API and dependencies',
  openGraph: {
    title: 'API Status | StellarRoute',
    description: 'Monitor the health of StellarRoute services in real-time',
    type: 'website',
  },
};

export default function StatusPage() {
  return (
    <main className="min-h-[calc(100vh-80px)] py-10 px-4 sm:px-6 lg:px-8">
      <div className="container mx-auto max-w-5xl">
        {/* Header */}
        <div className="mb-8 space-y-2">
          <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight">
            API Status
          </h1>
          <p className="text-muted-foreground text-lg">
            Real-time health monitoring of StellarRoute services and dependencies
          </p>
        </div>

        {/* Status Dashboard */}
        <StatusDashboard />
      </div>
    </main>
  );
}

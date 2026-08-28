import { StatusDashboard } from '@/components/status/StatusDashboard';
import { buildPageMetadata } from '@/lib/seo';

export const metadata = buildPageMetadata({
  title: 'API Status',
  description:
    'Live health status of the StellarRoute Stellar DEX aggregator API and dependencies',
  path: '/status',
});

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

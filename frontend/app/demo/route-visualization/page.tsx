'use client';

import { useState } from 'react';
import { RouteVisualization } from '@/components/shared/RouteVisualization';
import { TradeRouteDisplay } from '@/components/shared/TradeRouteDisplay';
import type { PathStep, PriceQuote } from '@/types';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { RotateCcw, AlertCircle, Layers } from 'lucide-react';

const singleHopPath: PathStep[] = [
  {
    from_asset: { asset_type: 'native' },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    },
    price: '0.0850',
    source: 'sdex',
  },
];

const multiHopPath: PathStep[] = [
  {
    from_asset: { asset_type: 'native' },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    },
    price: '0.0850',
    source: 'sdex',
  },
  {
    from_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'BTC',
      asset_issuer: 'GBVOL67TMUQBGL4TZYNMY3ZQ5WGQY4GP6GY5OI63WMH5XQ5XQ5XQ5XQ5',
    },
    price: '0.000015',
    source: 'amm:CDQR7XQJUGQP3VXV3YKQJMVXQXQXQXQXQXQXQXQXQXQXQXQXQXQXQXQX',
  },
];

const complexPath: PathStep[] = [
  {
    from_asset: { asset_type: 'native' },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    },
    price: '0.0850',
    source: 'sdex',
  },
  {
    from_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'EURC',
      asset_issuer: 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5',
    },
    price: '0.9200',
    source: 'amm:CDQR7XQJUGQP3VXV3YKQJMVXQXQXQXQXQXQXQXQXQXQXQXQXQXQXQXQX',
  },
  {
    from_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'EURC',
      asset_issuer: 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5',
    },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'BTC',
      asset_issuer: 'GBVOL67TMUQBGL4TZYNMY3ZQ5WGQY4GP6GY5OI63WMH5XQ5XQ5XQ5XQ5',
    },
    price: '0.000016',
    source: 'sdex',
  },
];

const splitRouteQuote: PriceQuote = {
  base_asset: { asset_type: 'native' },
  quote_asset: {
    asset_type: 'credit_alphanum4',
    asset_code: 'BTC',
    asset_issuer: 'GBVOL67TMUQBGL4TZYNMY3ZQ5WGQY4GP6GY5OI63WMH5XQ5XQ5XQ5XQ5',
  },
  amount: '1000',
  price: '0.000001277',
  total: '0.001277',
  quote_type: 'sell',
  timestamp: Date.now(),
  price_impact: '0.15',
  path: [],
  split_paths: [
    {
      allocation_bps: 6000,
      path: [
        {
          from_asset: { asset_type: 'native' },
          to_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'USDC',
            asset_issuer:
              'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
          },
          price: '0.0850',
          source: 'sdex',
        },
        {
          from_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'USDC',
            asset_issuer:
              'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
          },
          to_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'BTC',
            asset_issuer:
              'GBVOL67TMUQBGL4TZYNMY3ZQ5WGQY4GP6GY5OI63WMH5XQ5XQ5XQ5XQ5',
          },
          price: '0.000015',
          source: 'sdex',
        },
      ],
      output_amount: '0.000765',
    },
    {
      allocation_bps: 4000,
      path: [
        {
          from_asset: { asset_type: 'native' },
          to_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'BTC',
            asset_issuer:
              'GBVOL67TMUQBGL4TZYNMY3ZQ5WGQY4GP6GY5OI63WMH5XQ5XQ5XQ5XQ5',
          },
          price: '0.00000128',
          source:
            'amm:CDQR7XQJUGQP3VXV3YKQJMVXQXQXQXQXQXQXQXQXQXQXQXQXQXQXQXQX',
        },
      ],
      output_amount: '0.000512',
    },
  ],
};

export default function RouteVisualizationDemo() {
  const [activeTab, setActiveTab] = useState<string>('routes');
  const [isLoading, setIsLoading] = useState(false);
  const [showError, setShowError] = useState(false);
  const [activePreset, setActivePreset] = useState<
    'single' | 'multi' | 'complex'
  >('multi');

  const simulateLoading = () => {
    setIsLoading(true);
    const timer = setTimeout(() => setIsLoading(false), 1500);
    return () => clearTimeout(timer);
  };

  const currentPath =
    activePreset === 'single'
      ? singleHopPath
      : activePreset === 'multi'
        ? multiHopPath
        : complexPath;

  return (
    <div className="container mx-auto py-8 px-4 max-w-6xl space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4 border-b pb-6">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-3xl font-bold tracking-tight">
              Route Visualization Sandbox
            </h1>
            <span className="inline-flex items-center rounded-full border border-primary/40 px-2.5 py-0.5 text-xs font-semibold text-primary">
              Demo Only
            </span>
          </div>
          <p className="text-muted-foreground mt-1">
            Isolated visual harness for testing single-hop, multi-hop,
            split-routing, and fallback states without live API traffic.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            onClick={() => {
              setIsLoading(false);
              setShowError(false);
              setActivePreset('multi');
              setActiveTab('routes');
            }}
            variant="outline"
            size="sm"
            className="gap-1.5"
            data-testid="reset-sandbox-btn"
          >
            <RotateCcw className="h-3.5 w-3.5" />
            Reset Sandbox
          </Button>
        </div>
      </div>

      <Tabs
        value={activeTab}
        onValueChange={setActiveTab}
        className="space-y-6"
      >
        <TabsList className="grid w-full grid-cols-3 max-w-md">
          <TabsTrigger value="routes" className="gap-1.5">
            <Layers className="h-4 w-4" />
            Path Scenarios
          </TabsTrigger>
          <TabsTrigger value="split" className="gap-1.5">
            Split Routing
          </TabsTrigger>
          <TabsTrigger value="states" className="gap-1.5">
            <AlertCircle className="h-4 w-4" />
            Edge States
          </TabsTrigger>
        </TabsList>

        <TabsContent value="routes" className="space-y-6">
          <Card className="p-4 bg-muted/30 border">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold">Select Route Preset</h3>
                <p className="text-xs text-muted-foreground">
                  Inspect hop layout, venue badges, and price transitions across
                  different path complexities.
                </p>
              </div>
              <div className="flex gap-2">
                <Button
                  size="sm"
                  variant={activePreset === 'single' ? 'default' : 'outline'}
                  onClick={() => setActivePreset('single')}
                  data-testid="preset-single-hop"
                >
                  Single Hop (SDEX)
                </Button>
                <Button
                  size="sm"
                  variant={activePreset === 'multi' ? 'default' : 'outline'}
                  onClick={() => setActivePreset('multi')}
                  data-testid="preset-multi-hop"
                >
                  2-Hop (SDEX + AMM)
                </Button>
                <Button
                  size="sm"
                  variant={activePreset === 'complex' ? 'default' : 'outline'}
                  onClick={() => setActivePreset('complex')}
                  data-testid="preset-complex"
                >
                  3-Hop Mixed
                </Button>
              </div>
            </div>
          </Card>

          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs text-muted-foreground px-1">
              <span>Interactive Output</span>
              <span>Hops: {currentPath.length}</span>
            </div>
            <RouteVisualization
              path={currentPath}
              breakdown={{
                hops: currentPath.length,
                priceImpact: activePreset === 'complex' ? '0.24%' : '0.05%',
                totalFees: '0.00001 XLM',
              }}
            />
          </div>
        </TabsContent>

        <TabsContent value="split" className="space-y-6">
          <Card className="p-4 bg-muted/30 border">
            <h3 className="text-sm font-semibold mb-1">
              Dynamic Liquidity Split
            </h3>
            <p className="text-xs text-muted-foreground">
              Simulates a 60/40 allocation split across SDEX orderbook and
              Soroban AMM pool with proportional routing metrics.
            </p>
          </Card>

          <TradeRouteDisplay quote={splitRouteQuote} />
        </TabsContent>

        <TabsContent value="states" className="space-y-6">
          <Card className="p-4 bg-muted/30 border space-y-4">
            <div>
              <h3 className="text-sm font-semibold mb-1">
                Component Lifecycle Probes
              </h3>
              <p className="text-xs text-muted-foreground">
                Trigger mock loading skeletons, network error boundaries, and
                empty route states.
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                onClick={simulateLoading}
                variant="outline"
                size="sm"
                disabled={isLoading}
                data-testid="simulate-loading-btn"
              >
                {isLoading ? 'Simulating (1.5s)...' : 'Simulate Loading'}
              </Button>
              <Button
                onClick={() => setShowError(!showError)}
                variant="outline"
                size="sm"
                data-testid="toggle-error-btn"
              >
                {showError ? 'Clear Error' : 'Trigger Error State'}
              </Button>
            </div>
          </Card>

          <div className="grid gap-6 md:grid-cols-2">
            <Card className="p-4 space-y-3">
              <div className="flex items-center justify-between">
                <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  Simulated Loading State
                </h4>
                {isLoading && (
                  <span className="inline-flex items-center rounded-full bg-secondary px-2.5 py-0.5 text-xs font-semibold animate-pulse">
                    Active
                  </span>
                )}
              </div>
              <RouteVisualization path={[]} isLoading={isLoading} />
            </Card>

            <Card className="p-4 space-y-3">
              <div className="flex items-center justify-between">
                <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  Simulated Error State
                </h4>
                {showError && (
                  <span className="inline-flex items-center rounded-full bg-destructive/10 text-destructive border border-destructive/20 px-2.5 py-0.5 text-xs font-semibold">
                    Error Active
                  </span>
                )}
              </div>
              <RouteVisualization
                path={[]}
                error={
                  showError
                    ? 'Upstream RPC timeout: failed to fetch route steps'
                    : undefined
                }
              />
            </Card>

            <Card className="p-4 space-y-3 md:col-span-2">
              <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Empty / No Route Found State
              </h4>
              <RouteVisualization path={[]} />
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

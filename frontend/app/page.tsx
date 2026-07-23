import { DemoSwap } from "@/components/DemoSwap";

export default function Home() {
  return (
    <div className="container mx-auto px-4 py-8">
      <h1 className="text-3xl font-bold text-center">StellarRoute</h1>
      <p className="text-muted-foreground mt-2 text-center mb-12">
        DEX Aggregator - Frontend Ready
      </p>

      <DemoSwap />
    </div>
  );
}


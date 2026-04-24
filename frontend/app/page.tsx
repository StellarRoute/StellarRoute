import DemoSwap from "@/components/DemoSwap";

export default function Home() {
  return (
    <div className="container mx-auto px-4 py-8">
      <h1 className="text-3xl font-bold text-center">StellarRoute</h1>
      <p className="text-muted-foreground mt-2 text-center mb-12">
        DEX Aggregator - Frontend Ready
      </p>

      <DemoSwap />

      <div className="mt-12 text-center">
        <a 
          href="/quote-inspector" 
          className="text-primary hover:underline font-medium flex items-center justify-center gap-2"
        >
          View Cross-Venue Quote Inspector Demo
          <span className="text-xs bg-primary/10 px-2 py-0.5 rounded-full">New</span>
        </a>
      </div>
    </div>
  );
}


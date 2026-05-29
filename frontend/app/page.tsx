import { DemoSwap } from "@/components/DemoSwap";
import { HeroSection } from "@/components/HeroSection";

export default function Home() {
  return (
    <div className="min-h-screen">
      {/* Hero Section with CTA */}
      <HeroSection />

      {/* Demo Section */}
      <div className="container mx-auto px-4 py-12">
        <div className="text-center mb-8">
          <h2 className="text-2xl font-bold mb-2">Try It Now</h2>
          <p className="text-muted-foreground">
            Experience the power of aggregated liquidity
          </p>
        </div>

        <DemoSwap />

        <div className="mt-12 text-center">
          <a 
            href="/quote-inspector" 
            className="text-primary hover:underline font-medium inline-flex items-center justify-center gap-2"
          >
            View Cross-Venue Quote Inspector Demo
            <span className="text-xs bg-primary/10 px-2 py-0.5 rounded-full">New</span>
          </a>
        </div>
      </div>
    </div>
  );
}


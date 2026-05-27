'use client';

import { useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { ArrowRight, TrendingUp, Zap, Shield } from 'lucide-react';
import Link from 'next/link';
import { cn } from '@/lib/utils';
import { useReducedMotion } from '@/hooks/useReducedMotion';

export function HeroSection() {
  // Featured pair: XLM to USDC
  const featuredPair = {
    from: 'native',
    to: 'USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    amount: '100',
  };

  const prefersReducedMotion = useReducedMotion();

  // Generate URL with timestamp - useMemo to avoid calling Date.now() on every render
  const swapUrl = useMemo(
    () =>
      `/swap?from=${encodeURIComponent(featuredPair.from)}&to=${encodeURIComponent(featuredPair.to)}&amount=${featuredPair.amount}&ts=${Date.now()}`,
    [featuredPair.from, featuredPair.to, featuredPair.amount]
  );

  return (
    <section className="relative overflow-hidden py-20 sm:py-28 lg:py-32">
      {/* Background Gradients */}
      <div className="absolute inset-0 -z-10">
        <div
          data-testid="hero-gradient-1"
          className={cn(
            'absolute top-0 left-1/4 w-96 h-96 bg-primary/20 rounded-full blur-3xl',
            !prefersReducedMotion && 'animate-pulse'
          )}
        />
        <div
          data-testid="hero-gradient-2"
          className={cn(
            'absolute bottom-0 right-1/4 w-96 h-96 bg-blue-500/20 rounded-full blur-3xl',
            !prefersReducedMotion && 'animate-pulse delay-700'
          )}
        />
      </div>

      <div className="container mx-auto px-4">
        <div className="max-w-4xl mx-auto text-center space-y-8">
          {/* Badge */}
          <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-primary/10 border border-primary/20">
            <Zap className="h-4 w-4 text-primary" />
            <span className="text-sm font-semibold text-primary">
              Best Execution Across All Stellar DEXs
            </span>
          </div>

          {/* Main Heading */}
          <h1 className="text-4xl sm:text-5xl lg:text-6xl font-extrabold tracking-tight">
            <span className="bg-gradient-to-br from-foreground via-foreground/90 to-foreground/70 bg-clip-text text-transparent">
              Swap Smarter on Stellar
            </span>
          </h1>

          {/* Subheading */}
          <p className="text-lg sm:text-xl text-muted-foreground max-w-2xl mx-auto leading-relaxed">
            StellarRoute aggregates liquidity from SDEX and Soroban AMMs to find
            you the best rates with minimal slippage. Trade with confidence.
          </p>

          {/* CTA Buttons */}
          <div className="flex flex-col sm:flex-row items-center justify-center gap-4 pt-4">
            <Button
              asChild
              size="lg"
              className="group relative overflow-hidden rounded-xl px-8 py-6 text-base font-semibold shadow-lg hover:shadow-primary/25 transition-all duration-300"
            >
              <Link href={swapUrl}>
                <span className="relative z-10 flex items-center gap-2">
                  Start Trading XLM/USDC
                  <ArrowRight className="h-5 w-5 group-hover:translate-x-1 transition-transform" />
                </span>
                <div className="absolute inset-0 bg-gradient-to-r from-primary to-primary/80 group-hover:from-primary/90 group-hover:to-primary transition-all" />
              </Link>
            </Button>

            <Button
              asChild
              variant="outline"
              size="lg"
              className="rounded-xl px-8 py-6 text-base font-semibold border-border/40 hover:border-primary/40 hover:bg-primary/5 transition-all duration-300"
            >
              <Link href="/swap">
                Explore All Pairs
              </Link>
            </Button>
          </div>

          {/* Feature Pills */}
          <div className="flex flex-wrap items-center justify-center gap-6 pt-8 text-sm">
            <div className="flex items-center gap-2 text-muted-foreground">
              <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-emerald-500/10">
                <TrendingUp className="h-4 w-4 text-emerald-600 dark:text-emerald-400" />
              </div>
              <span className="font-medium">Best Rates</span>
            </div>

            <div className="flex items-center gap-2 text-muted-foreground">
              <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-blue-500/10">
                <Zap className="h-4 w-4 text-blue-600 dark:text-blue-400" />
              </div>
              <span className="font-medium">Instant Execution</span>
            </div>

            <div className="flex items-center gap-2 text-muted-foreground">
              <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-purple-500/10">
                <Shield className="h-4 w-4 text-purple-600 dark:text-purple-400" />
              </div>
              <span className="font-medium">Secure & Audited</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

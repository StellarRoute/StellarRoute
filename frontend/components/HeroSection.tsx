'use client';

import { Button } from '@/components/ui/button';
import {
  Activity,
  ArrowRight,
  CheckCircle2,
  CircleIcon,
  Compass,
  Layers,
  Lock,
  Route,
  ShieldCheck,
  Wallet,
} from 'lucide-react';
import Link from 'next/link';
import { cn } from '@/lib/utils';
import { useReducedMotion } from '@/hooks/useReducedMotion';

const routeSteps = [
  {
    label: 'Stellar wallet',
    detail: 'You authorize',
    icon: Wallet,
  },
  {
    label: 'SDEX + Soroban',
    detail: 'Best route selected',
    icon: Layers,
  },
  {
    label: 'Circle CCTP',
    detail: 'Native USDC burn',
    icon: Compass,
  },
  {
    label: 'Sepolia',
    detail: 'USDC minted',
    icon: CircleIcon,
  },
];

const capabilities = [
  {
    eyebrow: '01 / ROUTE',
    title: 'Stellar liquidity routing',
    description:
      'Compare executable liquidity across the Stellar SDEX and Soroban AMMs from one route engine.',
    icon: Route,
  },
  {
    eyebrow: '02 / SIGN',
    title: 'Non-custodial execution',
    description:
      'StellarRoute builds the path. Your connected wallet reviews and signs the execution.',
    icon: Lock,
  },
  {
    eyebrow: '03 / BRIDGE',
    title: 'Cross-chain USDC',
    description:
      'Circle CCTP now carries native USDC both ways between Stellar Testnet and Ethereum Sepolia.',
    icon: Compass,
  },
];

export function HeroSection() {
  const prefersReducedMotion = useReducedMotion();

  const reveal = (delay = '') =>
    !prefersReducedMotion &&
    cn('animate-in fade-in slide-in-from-bottom-3 duration-700', delay);

  return (
    <div className="relative isolate overflow-hidden">
      {/* Chart atmosphere layers — keep testids for reduced-motion suite. */}
      <div className="absolute inset-0 -z-10">
        <div
          data-testid="hero-gradient-1"
          className={cn(
            'absolute -left-40 top-20 h-[36rem] w-[36rem] rounded-full bg-primary/10 blur-3xl',
            !prefersReducedMotion && 'animate-pulse'
          )}
        />
        <div
          data-testid="hero-gradient-2"
          className={cn(
            'absolute -right-48 top-[36rem] h-[32rem] w-[32rem] rounded-full bg-signal/10 blur-3xl',
            !prefersReducedMotion && 'animate-pulse delay-700'
          )}
        />
        <div
          className={cn(
            'absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/60 to-transparent',
            !prefersReducedMotion && 'animate-in fade-in duration-700'
          )}
        />
      </div>

      <section
        aria-labelledby="landing-title"
        className="container mx-auto px-4 pb-24 pt-14 sm:px-6 sm:pb-32 sm:pt-20 lg:px-8 lg:pt-24"
      >
        <div className="grid items-start gap-16 lg:grid-cols-[minmax(0,1.03fr)_minmax(28rem,0.97fr)] lg:gap-12">
          <div className="max-w-3xl">
            <div
              className={cn(
                'mb-10 flex flex-wrap items-center gap-3',
                reveal()
              )}
            >
              <span className="inline-flex items-center gap-2 rounded-full border border-signal/60 bg-signal/10 px-3 py-1.5 font-mono text-[0.68rem] font-semibold tracking-[0.18em] text-foreground">
                <span className="h-2 w-2 rounded-full bg-signal" aria-hidden="true" />
                TESTNET CORRIDOR
              </span>
              <span className="font-mono text-[0.68rem] uppercase tracking-[0.16em] text-muted-foreground">
                Stellar ↔ Ethereum Sepolia
              </span>
            </div>

            <p
              className={cn(
                'mb-5 font-mono text-xs font-medium uppercase tracking-[0.24em] text-primary',
                reveal('delay-100')
              )}
            >
              Non-custodial Stellar DEX aggregator
            </p>
            <h1
              id="landing-title"
              className={cn(
                'font-display max-w-3xl text-5xl font-semibold leading-[0.98] tracking-[-0.055em] text-foreground sm:text-6xl lg:text-[5rem]',
                reveal('delay-150')
              )}
            >
              Stellar DEX aggregator.{' '}
              <span className="text-muted-foreground">
                Cross-chain swaps beyond it.
              </span>
            </h1>
            <p
              className={cn(
                'mt-7 max-w-2xl text-lg leading-relaxed text-muted-foreground sm:text-xl',
                reveal('delay-300')
              )}
            >
              Best-price routing across the Stellar DEX (SDEX) and Soroban AMMs,
              plus cross-chain USDC swaps via Circle CCTP — without giving up
              custody.
            </p>

            <div
              className={cn(
                'mt-9 flex flex-col gap-4 sm:flex-row sm:items-center',
                reveal('delay-500')
              )}
            >
              <Button
                asChild
                size="lg"
                className="h-12 min-h-11 rounded-lg px-7 text-base font-semibold"
              >
                <Link href="/swap">
                  Open execution deck
                  <ArrowRight className="h-5 w-5" aria-hidden="true" />
                </Link>
              </Button>
              <a
                href="#live-proof"
                className="inline-flex min-h-11 items-center justify-center gap-2 rounded-lg px-4 text-sm font-semibold text-foreground underline decoration-border underline-offset-8 hover:decoration-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              >
                Inspect signed-live proof
              </a>
            </div>
            <p
              className={cn(
                'mt-5 text-sm text-muted-foreground',
                reveal('delay-500')
              )}
            >
              Learn more:{' '}
              <Link
                href="/stellar-dex-aggregator"
                className="font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
              >
                Stellar DEX aggregator
              </Link>
              {' · '}
              <Link
                href="/cross-chain-swap"
                className="font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
              >
                Cross-chain swap
              </Link>
            </p>

            <div
              className={cn(
                'mt-10 flex items-start gap-3 border-l-2 border-signal/70 pl-4 text-sm leading-relaxed text-muted-foreground',
                reveal('delay-500')
              )}
            >
              <ShieldCheck
                className="mt-0.5 h-4 w-4 shrink-0 text-signal"
                aria-hidden="true"
              />
              <p>
                <strong className="font-semibold text-foreground">
                  Testnet proof, production gated.
                </strong>{' '}
                CCTP is disabled by default at the API layer until an operator
                configures it.
              </p>
            </div>
          </div>

          <div
            className={cn(
              'relative lg:mt-9',
              reveal('delay-300')
            )}
          >
            <div className="chart-panel relative overflow-hidden rounded-2xl p-5 sm:p-7">
              <div
                className="pointer-events-none absolute inset-0 opacity-40"
                aria-hidden="true"
                style={{
                  backgroundImage:
                    'linear-gradient(color-mix(in srgb, var(--border) 45%, transparent) 1px, transparent 1px), linear-gradient(90deg, color-mix(in srgb, var(--border) 45%, transparent) 1px, transparent 1px)',
                  backgroundSize: '32px 32px',
                  maskImage:
                    'linear-gradient(to bottom, black, transparent 90%)',
                }}
              />

              <div className="relative flex items-center justify-between border-b border-border/70 pb-5">
                <div>
                  <p className="font-mono text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">
                    Route constellation / 001
                  </p>
                  <p className="mt-1 font-display text-lg font-semibold text-foreground">
                    USDC corridor / both ways
                  </p>
                </div>
                <span className="rounded-md border border-signal/60 bg-signal/10 px-2 py-1 font-mono text-[0.6rem] font-semibold tracking-[0.16em] text-foreground">
                  TESTNET
                </span>
              </div>

              <div
                role="img"
                aria-label="Testnet Circle CCTP corridor between Stellar and Ethereum Sepolia, with Stellar wallet, SDEX and Soroban routing, and destination mint"
                className="relative mt-8 grid gap-7 md:grid-cols-4 md:gap-3"
              >
                <div
                  className="absolute bottom-5 left-5 top-5 w-px bg-gradient-to-b from-primary via-primary to-signal md:bottom-auto md:left-[12.5%] md:right-[12.5%] md:top-5 md:h-px md:w-auto"
                  aria-hidden="true"
                />
                {routeSteps.map((step, index) => {
                  const Icon = step.icon;
                  return (
                    <div
                      key={step.label}
                      className="relative z-10 flex min-w-0 items-center gap-4 md:flex-col md:items-center md:gap-3 md:text-center"
                    >
                      <div
                        className={cn(
                          'flex h-10 w-10 shrink-0 items-center justify-center rounded-full border bg-card',
                          index < 2
                            ? 'border-primary/70 text-primary'
                            : 'border-signal/70 text-signal'
                        )}
                      >
                        <Icon className="h-4 w-4" aria-hidden="true" />
                      </div>
                      <div className="min-w-0">
                        <p className="text-sm font-semibold text-foreground">
                          {step.label}
                        </p>
                        <p className="mt-0.5 font-mono text-[0.6rem] uppercase tracking-[0.08em] text-muted-foreground">
                          {step.detail}
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>

              <div className="relative mt-8 grid grid-cols-2 border-t border-border/70 pt-5">
                <div>
                  <p className="font-mono text-[0.6rem] uppercase tracking-[0.14em] text-muted-foreground">
                    Source domain
                  </p>
                  <p className="mt-1 text-sm font-semibold text-foreground">
                    Stellar Testnet
                  </p>
                </div>
                <div className="border-l border-border/70 pl-5">
                  <p className="font-mono text-[0.6rem] uppercase tracking-[0.14em] text-muted-foreground">
                    Destination
                  </p>
                  <p className="mt-1 text-sm font-semibold text-foreground">
                    Ethereum Sepolia
                  </p>
                </div>
              </div>
            </div>

            <div
              className="absolute -bottom-5 -right-2 hidden items-center gap-2 border border-border bg-background px-3 py-2 font-mono text-[0.6rem] uppercase tracking-[0.14em] text-muted-foreground shadow-lg sm:flex"
              aria-hidden="true"
            >
              <CheckCircle2 className="h-3.5 w-3.5 text-primary" />
              signed route observed
            </div>
          </div>
        </div>
      </section>

      <section
        aria-labelledby="capabilities-title"
        className="border-y border-border/70 bg-card/35"
      >
        <div className="container mx-auto px-4 py-24 sm:px-6 sm:py-28 lg:px-8">
          <div className="grid gap-10 lg:grid-cols-[0.72fr_1.28fr] lg:gap-20">
            <div>
              <p className="font-mono text-xs font-medium uppercase tracking-[0.22em] text-primary">
                Execution coordinates
              </p>
              <h2
                id="capabilities-title"
                className="font-display mt-4 max-w-md text-4xl font-semibold leading-tight tracking-[-0.04em] sm:text-5xl"
              >
                One intent. Three critical systems.
              </h2>
            </div>

            <div>
              {capabilities.map((capability) => {
                const Icon = capability.icon;
                return (
                  <article
                    key={capability.title}
                    className="grid gap-4 border-t border-border py-7 first:pt-5 sm:grid-cols-[7rem_1fr_auto] sm:items-start sm:gap-6"
                  >
                    <p className="font-mono text-[0.64rem] font-semibold tracking-[0.16em] text-primary">
                      {capability.eyebrow}
                    </p>
                    <div>
                      <h3 className="font-display text-xl font-semibold tracking-tight text-foreground">
                        {capability.title}
                      </h3>
                      <p className="mt-2 max-w-xl leading-relaxed text-muted-foreground">
                        {capability.description}
                      </p>
                    </div>
                    <div className="hidden h-10 w-10 items-center justify-center rounded-full border border-border text-muted-foreground sm:flex">
                      <Icon className="h-4 w-4" aria-hidden="true" />
                    </div>
                  </article>
                );
              })}
            </div>
          </div>
        </div>
      </section>

      <section
        id="live-proof"
        aria-labelledby="proof-title"
        className="container mx-auto scroll-mt-24 px-4 py-24 sm:px-6 sm:py-32 lg:px-8"
      >
        <div className="grid gap-14 lg:grid-cols-[0.8fr_1.2fr] lg:items-end lg:gap-24">
          <div>
            <div className="inline-flex items-center gap-2 border border-primary/40 bg-primary/10 px-3 py-1.5 font-mono text-[0.66rem] font-semibold uppercase tracking-[0.16em] text-foreground">
              <CheckCircle2 className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
              Signed-live / testnet evidence
            </div>
            <h2
              id="proof-title"
              className="font-display mt-7 text-4xl font-semibold leading-tight tracking-[-0.04em] sm:text-5xl"
            >
              The corridor completed, not just compiled.
            </h2>
            <p className="mt-5 max-w-xl text-lg leading-relaxed text-muted-foreground">
              Wallet-signed runs now prove both directions: Stellar Testnet →
              Sepolia and Sepolia → Stellar Testnet, each with Circle
              attestation and a destination mint.
            </p>
            <p className="mt-4 font-mono text-xs leading-relaxed text-muted-foreground break-all">
              Stellar → Sepolia mint:{' '}
              <a
                href="https://sepolia.etherscan.io/tx/0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6"
                className="text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
                target="_blank"
                rel="noopener noreferrer"
              >
                0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6
              </a>
            </p>
            <p className="mt-2 font-mono text-xs leading-relaxed text-muted-foreground break-all">
              Sepolia → Stellar mint:{' '}
              <a
                href="https://stellar.expert/explorer/testnet/tx/13d2025db39b461756954e1266864ea39c126cada55ddf24db9ec364138d16f2"
                className="text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
                target="_blank"
                rel="noopener noreferrer"
              >
                13d2025db39b461756954e1266864ea39c126cada55ddf24db9ec364138d16f2
              </a>
            </p>
            <div className="mt-8 flex items-start gap-3 border-l-2 border-signal pl-4">
              <Activity
                className="mt-0.5 h-4 w-4 shrink-0 text-signal"
                aria-hidden="true"
              />
              <p className="text-sm leading-relaxed text-muted-foreground">
                Both directions are signed-live on testnet. This does not claim
                mainnet availability; public CCTP remains operator-gated.
              </p>
            </div>
          </div>

          <dl className="grid grid-cols-1 gap-px overflow-hidden border border-border bg-border sm:grid-cols-2">
            <div className="bg-card p-7 sm:p-9">
              <dt className="font-mono text-[0.66rem] uppercase tracking-[0.18em] text-muted-foreground">
                Total saga
              </dt>
              <dd className="font-display mt-8 text-7xl font-semibold leading-none tracking-[-0.06em] text-foreground sm:text-8xl">
                63<span className="ml-1 text-2xl text-primary">s</span>
              </dd>
              <p className="mt-5 text-sm text-muted-foreground">
                Stellar Testnet → Sepolia completion
              </p>
            </div>
            <div className="bg-card p-7 sm:p-9">
              <dt className="font-mono text-[0.66rem] uppercase tracking-[0.18em] text-muted-foreground">
                Burn → attestation
              </dt>
              <dd className="font-display mt-8 text-7xl font-semibold leading-none tracking-[-0.06em] text-foreground sm:text-8xl">
                33<span className="ml-1 text-2xl text-signal">s</span>
              </dd>
              <p className="mt-5 text-sm text-muted-foreground">
                Circle Iris attestation observed
              </p>
            </div>
          </dl>
        </div>
      </section>

      <section className="container mx-auto px-4 pb-12 sm:px-6 sm:pb-16 lg:px-8">
        <div className="relative overflow-hidden border border-primary/40 bg-foreground px-6 py-12 text-background sm:px-10 sm:py-14 lg:px-14">
          <div
            className="absolute -right-20 -top-28 h-80 w-80 rounded-full border border-background/15"
            aria-hidden="true"
          />
          <div
            className="absolute -right-4 -top-10 h-44 w-44 rounded-full border border-background/15"
            aria-hidden="true"
          />
          <div className="relative flex flex-col items-start justify-between gap-8 lg:flex-row lg:items-end">
            <div>
              <p className="font-mono text-[0.68rem] font-semibold uppercase tracking-[0.2em] text-primary">
                Ready on Stellar
              </p>
              <h2 className="font-display mt-4 max-w-2xl text-3xl font-semibold leading-tight tracking-[-0.035em] sm:text-4xl">
                Plot the route. Keep the signature.
              </h2>
            </div>
            <Button
              asChild
              size="lg"
              className="h-12 min-h-11 shrink-0 rounded-lg bg-primary px-7 text-base text-primary-foreground hover:bg-primary/90"
            >
              <Link href="/swap">
                Open execution deck
                <ArrowRight className="h-5 w-5" aria-hidden="true" />
              </Link>
            </Button>
          </div>
        </div>
      </section>
    </div>
  );
}

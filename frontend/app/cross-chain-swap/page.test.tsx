import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import CrossChainSwapPage, { metadata } from './page';

/**
 * `/cross-chain-swap` is a static server component. It is the public SEO
 * surface for the CCTP corridor, and it is deliberately *not* the execution
 * surface — execution lives behind the gated `/swap` cross-chain deck.
 *
 * This suite locks that separation: the page must never grow a burn/mint CTA,
 * and it must keep describing mainnet enablement as operator-gated. It also
 * pins `CCTP_ENABLED` to default-false in the shipped example env files, which
 * is the freeze the issue calls out.
 */

function readRepoFile(relativePath: string): string {
  return readFileSync(fileURLToPath(new URL(relativePath, import.meta.url)), 'utf8');
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('CrossChainSwapPage metadata', () => {
  it('keeps the SEO title, description and canonical path', () => {
    expect(metadata.title).toBe('Cross-Chain Swap on Stellar');
    expect(metadata.description).toContain('cross-chain USDC swaps');
    expect(metadata.description).toContain('Circle CCTP');
  });

  it('publishes FAQ structured data for the corridor', () => {
    render(<CrossChainSwapPage />);

    const jsonLd = document.querySelector('script[type="application/ld+json"]');
    expect(jsonLd).not.toBeNull();

    const parsed = JSON.parse(jsonLd!.textContent ?? '{}');
    expect(JSON.stringify(parsed)).toContain('What is a Stellar cross-chain swap?');
  });
});

describe('CrossChainSwapPage never renders an execution CTA', () => {
  it('has no button elements at all — only navigation links', () => {
    render(<CrossChainSwapPage />);

    // The page is a marketing surface: it links into the gated deck, it never
    // executes. Any <button> here is a regression.
    expect(document.querySelectorAll('button')).toHaveLength(0);
  });

  it('exposes no burn, mint, quote or confirm control', () => {
    render(<CrossChainSwapPage />);

    // The CCTP burn/review CTA shared by the deck lives on testid
    // `cross-chain-review-cta`; assert it can never appear on this route.
    expect(screen.queryByTestId('cross-chain-review-cta')).not.toBeInTheDocument();
    expect(screen.queryByTestId('cctp-execution-panel')).not.toBeInTheDocument();
    expect(screen.queryByTestId('cross-chain-swap-deck')).not.toBeInTheDocument();
    expect(screen.queryByTestId('cctp-abandon-cta')).not.toBeInTheDocument();

    // And no control may claim to burn, mint, prepare, approve or confirm.
    const actionable = /burn|mint|quote|confirm|prepare|approve|transfer now|swap now/i;
    for (const control of document.querySelectorAll('a, button, input, [role="button"]')) {
      const label = `${control.textContent ?? ''} ${control.getAttribute('aria-label') ?? ''}`;
      expect(label).not.toMatch(actionable);
    }
  });

  it('makes no API call while rendering, so it cannot be enabled by data', () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');

    render(<CrossChainSwapPage />);

    // A server component that never probes readiness cannot light itself up
    // from a live `/api/v2` response.
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

describe('CrossChainSwapPage gated copy', () => {
  it('describes mainnet enablement as operator-gated, not live', () => {
    render(<CrossChainSwapPage />);

    expect(
      screen.getByText(/public mainnet enablement operator-gated/i),
    ).toBeInTheDocument();
  });

  it('scopes the corridor to testnet and describes burn/mint as a flow, not an action', () => {
    render(<CrossChainSwapPage />);

    // The corridor copy must keep naming the testnet pair it actually supports.
    expect(screen.getByText(/Stellar Testnet and Ethereum Sepolia/i)).toBeInTheDocument();

    // Burn/mint appear only inside explanatory prose, never as a control label.
    expect(screen.getByText(/burn\/mint path, fees, and attestation wait/i)).toBeInTheDocument();
    expect(screen.getByText(/Authorize the Stellar burn \(or EVM burn\)/i)).toBeInTheDocument();
  });

  it('routes every action out to the gated deck or docs, never to an executor', () => {
    render(<CrossChainSwapPage />);

    expect(screen.getByRole('link', { name: /Open cross-chain deck/i })).toHaveAttribute(
      'href',
      '/swap',
    );
    expect(
      screen.getByRole('link', { name: /Stellar DEX aggregator/i }),
    ).toHaveAttribute('href', '/stellar-dex-aggregator');
    expect(screen.getByRole('link', { name: /first live swap guide/i })).toHaveAttribute(
      'href',
      '/guide',
    );

    // Nothing on this page posts, signs or burns.
    for (const link of document.querySelectorAll('a')) {
      expect(link.getAttribute('href')).toMatch(/^\/(swap|stellar-dex-aggregator|guide)$/);
    }
  });
});

describe('CCTP_ENABLED default stays off in the shipped examples', () => {
  it.each([
    ['root .env.example', '../../../.env.example'],
    ['deploy .env.prod.example', '../../../deploy/env.prod.example'],
  ])('%s keeps CCTP_ENABLED commented out or false', (_label, relativePath) => {
    const contents = readRepoFile(relativePath);

    const active = contents
      .split('\n')
      .map((line) => line.trim())
      // Ignore comments: the freeze is that no example *sets* CCTP_ENABLED.
      .filter((line) => line.startsWith('CCTP_ENABLED='));

    expect(active).toEqual([]);
  });

  it('never exposes a NEXT_PUBLIC CCTP flag to the browser bundle', () => {
    const frontendEnv = readRepoFile('../../.env.example');

    // The frontend cannot read the server flag, so the page can never be
    // switched on from a client bundle.
    expect(frontendEnv).not.toMatch(/NEXT_PUBLIC_\w*CCTP/);
  });
});

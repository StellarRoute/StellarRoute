import React from 'react';
import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import AgentGuideSection from './agent-guide-section';
import FirstSwapGuidePage from './page';

function clearFlag() {
  delete process.env.NEXT_PUBLIC_AI_AGENT;
  delete process.env.NEXT_PUBLIC_FLAGS_URL;
  delete (
    window as unknown as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> }
  ).__STELLAR_ROUTE_FLAGS__;
}

describe('AgentGuideSection (AI-44 #1454)', () => {
  afterEach(() => {
    clearFlag();
  });

  it('flag off: renders nothing with no Agent heading', () => {
    clearFlag();
    const { container } = render(<AgentGuideSection />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId('guide-agent-section')).not.toBeInTheDocument();
  });

  it('flag on via env: shows Agent heading with confirm-before-sign', async () => {
    process.env.NEXT_PUBLIC_AI_AGENT = 'true';
    render(<AgentGuideSection />);
    await waitFor(() => {
      expect(screen.getByTestId('guide-agent-section')).toBeInTheDocument();
    });
    expect(screen.getByRole('heading', { name: /agent/i })).toBeInTheDocument();
    expect(screen.getByText(/confirm-before-sign/i)).toBeInTheDocument();
  });

  it('flag on via window override: shows section', async () => {
    clearFlag();
    (
      window as unknown as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> }
    ).__STELLAR_ROUTE_FLAGS__ = { ai_agent: true };
    render(<AgentGuideSection />);
    await waitFor(() => {
      expect(screen.getByTestId('guide-agent-section')).toBeInTheDocument();
    });
  });

  it('guide page keeps existing sections when flag is off', () => {
    clearFlag();
    render(<FirstSwapGuidePage />);
    expect(
      screen.getByRole('heading', { name: /your first live swap/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: /connect your wallet/i }),
    ).toBeInTheDocument();
    expect(screen.queryByTestId('guide-agent-section')).not.toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: /agent/i })).not.toBeInTheDocument();
  });
});

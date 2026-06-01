import { render, screen, cleanup } from '@testing-library/react';
import { Footer } from './footer';
import { describe, it, expect, vi, afterEach } from 'vitest';

// Mock the wallet provider
vi.mock('@/components/providers/wallet-provider', () => ({
  useWallet: () => ({
    network: 'testnet',
  }),
}));

// Mock Next.js Link
vi.mock('next/link', () => ({
  default: ({ children, href }: { children: React.ReactNode; href: string }) => (
    <a href={href}>{children}</a>
  ),
}));

describe('Footer', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders all footer links', () => {
    render(<Footer />);
    
    expect(screen.getByRole('link', { name: /Status/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /GitHub/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Docs/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Stellar\.org/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Community/i })).toBeInTheDocument();
  });

  it('renders status link as internal link', () => {
    render(<Footer />);
    
    const statusLink = screen.getByRole('link', { name: /Status/i });
    expect(statusLink).toHaveAttribute('href', '/status');
    // Internal links should not have target="_blank"
    expect(statusLink).not.toHaveAttribute('target', '_blank');
  });

  it('renders external links with correct attributes', () => {
    render(<Footer />);
    
    const githubLink = screen.getByRole('link', { name: /GitHub/i });
    expect(githubLink).toHaveAttribute('target', '_blank');
    expect(githubLink).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('displays network badge', () => {
    render(<Footer />);
    
    expect(screen.getAllByText('Testnet')[0]).toBeInTheDocument();
  });

  it('displays "Built for Stellar" text', () => {
    render(<Footer />);
    
    expect(screen.getAllByText(/Built for/i)[0]).toBeInTheDocument();
    // Get all links with Stellar and check the one without ".org"
    const stellarLinks = screen.getAllByRole('link', { name: /Stellar/i });
    const builtForLink = stellarLinks.find(link => link.textContent === 'Stellar');
    expect(builtForLink).toBeInTheDocument();
  });

  it('has proper navigation landmark', () => {
    render(<Footer />);
    
    const navs = screen.getAllByRole('navigation', { name: /Footer navigation/i });
    expect(navs[0]).toBeInTheDocument();
  });

  it('status link appears first in the list', () => {
    render(<Footer />);
    
    const links = screen.getAllByRole('link');
    const footerLinks = links.filter(link => 
      ['Status', 'GitHub', 'Docs', 'Stellar.org', 'Community'].some(text => 
        link.textContent?.includes(text)
      )
    );
    
    expect(footerLinks[0]).toHaveTextContent('Status');
  });
});

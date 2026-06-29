import { ExternalLink } from "lucide-react";
import type { Metadata } from "next";

const docsLinks = [
  {
    title: "Documentation Index",
    description: "Browse the main StellarRoute documentation directory.",
    href: "https://github.com/StellarRoute/StellarRoute/tree/main/docs",
  },
  {
    title: "API Reference",
    description: "Review REST API routes, schemas, websocket docs, and integration guidance.",
    href: "https://github.com/StellarRoute/StellarRoute/tree/main/docs/api",
  },
  {
    title: "Developer Guide",
    description: "Set up the frontend, indexer, testing flow, and wallet integration locally.",
    href: "https://github.com/StellarRoute/StellarRoute/tree/main/docs/development",
  },
  {
    title: "Contract Docs",
    description: "Read contract deployment, testing, router interface, and gas benchmark notes.",
    href: "https://github.com/StellarRoute/StellarRoute/tree/main/docs/contracts",
  },
];

export const metadata: Metadata = {
  title: "Docs | StellarRoute",
  description: "StellarRoute documentation links for API, development, and contract guides",
};

export default function DocsPage() {
  return (
    <main className="min-h-[calc(100vh-80px)] px-4 py-10 sm:px-6 lg:px-8">
      <div className="container mx-auto max-w-5xl">
        <div className="mb-8 space-y-2">
          <p className="text-sm font-medium uppercase text-muted-foreground">
            Documentation
          </p>
          <h1 className="text-3xl font-extrabold tracking-tight sm:text-4xl">
            StellarRoute Docs
          </h1>
          <p className="max-w-2xl text-lg text-muted-foreground">
            Jump into the project guides, API references, and contract documentation
            hosted in the StellarRoute repository.
          </p>
        </div>

        <div className="grid gap-4 sm:grid-cols-2">
          {docsLinks.map((link) => (
            <a
              key={link.href}
              href={link.href}
              target="_blank"
              rel="noopener noreferrer"
              className="rounded-lg border bg-card p-5 text-card-foreground transition-colors hover:border-primary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              <span className="mb-3 flex items-center justify-between gap-3">
                <span className="text-base font-semibold">{link.title}</span>
                <ExternalLink className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
              </span>
              <span className="block text-sm leading-6 text-muted-foreground">
                {link.description}
              </span>
            </a>
          ))}
        </div>
      </div>
    </main>
  );
}

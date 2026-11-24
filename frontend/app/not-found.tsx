import Link from "next/link";

/**
 * Next.js 404 not-found page.
 *
 * Shows a minimal message and a link back to /swap so users are never
 * stranded on a dead URL.
 */
export default function NotFound() {
  return (
    <main className="flex min-h-[60vh] flex-col items-center justify-center gap-4 px-4 text-center">
      <h1 className="text-4xl font-bold">404</h1>
      <p className="text-muted-foreground">Page not found.</p>
      <Link
        href="/swap"
        className="mt-2 inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        data-testid="not-found-swap-link"
      >
        Back to Swap
      </Link>
    </main>
  );
}

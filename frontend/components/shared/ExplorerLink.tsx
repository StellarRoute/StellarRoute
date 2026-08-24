import { ExternalLink } from "lucide-react";

interface ExplorerLinkProps {
  hash: string;
  className?: string;
}

export function ExplorerLink({ hash, className }: ExplorerLinkProps) {
  if (!hash) return null;

  return (
    <a
      href={`https://stellar.expert/explorer/public/tx/${hash}`}
      target="_blank"
      rel="noreferrer noopener"
      aria-label={`View transaction ${hash.slice(0, 8)} on Stellar Expert`}
      className={className}
    >
      View on Stellar Expert <ExternalLink className="inline-block h-3 w-3 ml-1" aria-hidden="true" />
    </a>
  );
}

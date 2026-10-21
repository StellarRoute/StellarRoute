/**
 * Card Cross-Chain Top-Up Proof of Deposit Receipt (CARD-51).
 *
 * Consolidates EVM source burn and Stellar destination mint metadata
 * into a single verifiable receipt for audits and troubleshooting.
 */

export interface DepositSagaState {
  sourceChain?: string;
  sourceChainId?: string;
  source_chain_id?: string;
  burnTxHash?: string;
  sourceTxHash?: string;
  source_tx_hash?: string;
  mintTxHash?: string;
  destTxHash?: string;
  destinationTxHash?: string;
  destination_tx_hash?: string;
  amount?: string | number;
  usdcAmount?: string | number;
  source_amount?: string | number;
  destination_amount?: string | number;
  timestamp?: number | string;
  createdAt?: number | string;
  recipient?: string;
  status?: string;
  quote?: {
    source_amount?: string;
    destination_amount?: string;
    source_chain_id?: string;
    destination_chain_id?: string;
    direction?: string;
    transfer_id?: string;
  } | null;
  transferStatus?: {
    source_tx_hash?: string;
    destination_tx_hash?: string;
    status?: string;
    transfer_id?: string;
    support_reference_id?: string;
    direction?: string;
  } | null;
  session?: {
    createdAt?: number;
    recovery?: {
      sourceChainId?: string;
      destChainId?: string;
      amount?: string;
      recipient?: string;
    };
  } | null;
}

export interface DepositReceipt {
  sourceChain: string;
  sourceNetworkBadge: string;
  burnTxHash: string;
  mintTxHash: string;
  amount: string;
  usdcAmount: string;
  timestamp: string;
  timestampFormatted: string;
  timestampMs: number;
  recipient?: string;
  status: string;
  sourceExplorerUrl: string;
  destinationExplorerUrl: string;
  markdownSummary: string;
  textSummary: string;
}

export interface FormatDepositReceiptParams {
  sagaState?: DepositSagaState | null;
}

/**
 * Shortens a blockchain transaction hash or public key for clean display.
 */
export function shortenHash(
  value: string,
  head = 10,
  tail = 8,
): string {
  if (!value) return '';
  if (value.length <= head + tail + 1) return value;
  return `${value.slice(0, head)}…${value.slice(-tail)}`;
}

/**
 * Shortens an account or contract address for clean display.
 */
export function shortenAddress(value: string): string {
  if (!value) return '';
  if (value.startsWith('0x') && value.length >= 12) {
    return `${value.slice(0, 8)}…${value.slice(-6)}`;
  }
  if (value.length > 18) {
    return `${value.slice(0, 6)}…${value.slice(-6)}`;
  }
  return value;
}

/**
 * Resolves a human-readable network label from chain id or network identifier.
 */
export function resolveChainLabel(chainIdOrName?: string): string {
  if (!chainIdOrName) return 'Ethereum Sepolia';
  const lower = chainIdOrName.toLowerCase();
  if (lower.includes('11155111') || lower.includes('sepolia')) {
    return 'Ethereum Sepolia';
  }
  if (lower.includes('arbitrum')) return 'Arbitrum Sepolia';
  if (lower.includes('base')) return 'Base Sepolia';
  if (lower.includes('optimism')) return 'Optimism Sepolia';
  if (lower.includes('polygon')) return 'Polygon Amoy';
  if (lower === 'ethereum' || lower === 'eth' || lower === 'eip155:1') {
    return 'Ethereum Mainnet';
  }
  if (lower.includes('stellar')) return 'Stellar Testnet';
  return chainIdOrName;
}

/**
 * Generates the block explorer URL for a given transaction hash.
 */
export function getExplorerUrl(
  hash: string,
  chainOrNetwork?: string,
  leg: 'source' | 'dest' = 'source',
): string {
  if (!hash) return '';
  const encoded = encodeURIComponent(hash);

  if (leg === 'dest') {
    // Destination for card top-up is Stellar Testnet USDC
    return `https://stellar.expert/explorer/testnet/tx/${encoded}`;
  }

  const network = (chainOrNetwork || '').toLowerCase();
  if (network.includes('arbitrum')) {
    return `https://sepolia.arbiscan.io/tx/${encoded}`;
  }
  if (network.includes('base')) {
    return `https://sepolia.basescan.org/tx/${encoded}`;
  }
  if (network.includes('stellar')) {
    return `https://stellar.expert/explorer/testnet/tx/${encoded}`;
  }

  // Default source EVM explorer is Ethereum Sepolia Etherscan
  return `https://sepolia.etherscan.io/tx/${encoded}`;
}

/**
 * Formats a clean Markdown receipt for clipboard export.
 */
export function generateReceiptMarkdown(data: {
  amount: string;
  sourceChain: string;
  timestamp: string;
  burnTxHash: string;
  mintTxHash: string;
  sourceExplorerUrl: string;
  destinationExplorerUrl: string;
  status: string;
  recipient?: string;
}): string {
  const shortBurn = data.burnTxHash ? shortenHash(data.burnTxHash) : 'N/A';
  const shortMint = data.mintTxHash ? shortenHash(data.mintTxHash) : 'N/A';

  const burnLink = data.burnTxHash && data.sourceExplorerUrl
    ? `[${shortBurn}](${data.sourceExplorerUrl}) (\`${data.burnTxHash}\`)`
    : `\`${data.burnTxHash || 'N/A'}\``;

  const mintLink = data.mintTxHash && data.destinationExplorerUrl
    ? `[${shortMint}](${data.destinationExplorerUrl}) (\`${data.mintTxHash}\`)`
    : `\`${data.mintTxHash || 'N/A'}\``;

  const lines = [
    '# Cross-Chain Top-Up Proof of Deposit',
    '',
    `- **Status**: ${data.status}`,
    `- **Bridged Amount**: ${data.amount} USDC`,
    `- **Source Network**: ${data.sourceChain}`,
    `- **Destination Network**: Stellar Testnet`,
    `- **Deposit Timestamp**: ${data.timestamp}`,
    `- **Source EVM Burn Tx**: ${burnLink}`,
    `- **Destination Stellar Mint Tx**: ${mintLink}`,
  ];

  if (data.recipient) {
    lines.push(`- **Recipient Address**: \`${data.recipient}\``);
  }

  return lines.join('\n');
}

/**
 * Formats a plain text receipt summary.
 */
export function generateReceiptText(data: {
  amount: string;
  sourceChain: string;
  timestamp: string;
  burnTxHash: string;
  mintTxHash: string;
  sourceExplorerUrl: string;
  destinationExplorerUrl: string;
  status: string;
  recipient?: string;
}): string {
  const lines = [
    '=== Cross-Chain Top-Up Proof of Deposit ===',
    `Status: ${data.status}`,
    `Bridged Amount: ${data.amount} USDC`,
    `Source Network: ${data.sourceChain}`,
    `Destination Network: Stellar Testnet`,
    `Deposit Timestamp: ${data.timestamp}`,
    `Source EVM Burn Tx: ${data.burnTxHash || 'N/A'}`,
    `Destination Stellar Mint Tx: ${data.mintTxHash || 'N/A'}`,
    `Source Explorer: ${data.sourceExplorerUrl || 'N/A'}`,
    `Destination Explorer: ${data.destinationExplorerUrl || 'N/A'}`,
  ];

  if (data.recipient) {
    lines.push(`Recipient Address: ${data.recipient}`);
  }

  return lines.join('\n');
}

/**
 * Aggregates deposit metadata from a cross-chain saga state into a unified receipt.
 */
export function formatDepositReceipt(
  params: FormatDepositReceiptParams = {},
): DepositReceipt {
  const state = params.sagaState ?? {};

  const sourceChainRaw =
    state.sourceChain ??
    state.sourceChainId ??
    state.source_chain_id ??
    state.quote?.source_chain_id ??
    state.session?.recovery?.sourceChainId ??
    'ethereum-sepolia';

  const sourceChain = resolveChainLabel(sourceChainRaw);
  const sourceNetworkBadge = sourceChain;

  const burnTxHash =
    state.burnTxHash ??
    state.sourceTxHash ??
    state.source_tx_hash ??
    state.transferStatus?.source_tx_hash ??
    '';

  const mintTxHash =
    state.mintTxHash ??
    state.destinationTxHash ??
    state.destTxHash ??
    state.destination_tx_hash ??
    state.transferStatus?.destination_tx_hash ??
    '';

  const rawAmount =
    state.amount ??
    state.usdcAmount ??
    state.destination_amount ??
    state.source_amount ??
    state.quote?.destination_amount ??
    state.quote?.source_amount ??
    state.session?.recovery?.amount ??
    '0.00';

  const amount = String(rawAmount).trim() || '0.00';
  const usdcAmount = amount.endsWith('USDC') ? amount : `${amount} USDC`;

  const rawTs =
    state.timestamp ??
    state.createdAt ??
    state.session?.createdAt;

  let timestampMs: number;
  if (typeof rawTs === 'number') {
    timestampMs = rawTs < 10000000000 ? rawTs * 1000 : rawTs;
  } else if (typeof rawTs === 'string') {
    const parsed = Date.parse(rawTs);
    timestampMs = isNaN(parsed) ? Date.now() : parsed;
  } else {
    timestampMs = Date.now();
  }

  const dateObj = new Date(timestampMs);
  const timestamp = dateObj.toISOString();
  const timestampFormatted = dateObj.toUTCString();

  const recipient =
    state.recipient ??
    state.session?.recovery?.recipient ??
    undefined;

  const status =
    state.status ??
    state.transferStatus?.status ??
    'completed';

  const sourceExplorerUrl = getExplorerUrl(burnTxHash, sourceChainRaw, 'source');
  const destinationExplorerUrl = getExplorerUrl(mintTxHash, 'stellar', 'dest');

  const markdownSummary = generateReceiptMarkdown({
    amount,
    sourceChain,
    timestamp: timestampFormatted,
    burnTxHash,
    mintTxHash,
    sourceExplorerUrl,
    destinationExplorerUrl,
    status,
    recipient,
  });

  const textSummary = generateReceiptText({
    amount,
    sourceChain,
    timestamp: timestampFormatted,
    burnTxHash,
    mintTxHash,
    sourceExplorerUrl,
    destinationExplorerUrl,
    status,
    recipient,
  });

  return {
    sourceChain,
    sourceNetworkBadge,
    burnTxHash,
    mintTxHash,
    amount,
    usdcAmount,
    timestamp,
    timestampFormatted,
    timestampMs,
    recipient,
    status,
    sourceExplorerUrl,
    destinationExplorerUrl,
    markdownSummary,
    textSummary,
  };
}

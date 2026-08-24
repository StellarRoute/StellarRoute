import { StrKey } from '@stellar/stellar-base';
import type { CctpDirection, PreparedWalletPayload } from './types';
import {
  STELLAR_TESTNET_PASSPHRASE,
} from './types';
import type { CctpWalletRoles } from '@/hooks/useCctpSaga';

export const WALLET_BINDINGS_SCHEMA_VERSION = 1;

export type StellarRecipientKind = 'stellar_g' | 'stellar_m';
export type SignerBindingMode = 'required' | 'permissionless';

export interface BoundSourceBurnSigner {
  address: string;
  chainId: string;
  adapterFamily: 'evm' | 'stellar';
}

export interface BoundStellarMintSubmitter {
  address: string;
}

export interface BoundEvmMintSubmitter {
  mode: 'permissionless';
  chainId: string;
}

export interface CctpWalletRoleBindings {
  schemaVersion: typeof WALLET_BINDINGS_SCHEMA_VERSION;
  direction: CctpDirection;
  recipient: {
    kind: 'evm' | StellarRecipientKind;
    address: string;
  };
  sourceBurn: BoundSourceBurnSigner;
  stellarMintSubmitter?: BoundStellarMintSubmitter;
  evmMintSubmitter?: BoundEvmMintSubmitter;
}

export type WalletRoleMismatchCode =
  | 'bindings_missing'
  | 'source_burn_mismatch'
  | 'stellar_mint_submitter_mismatch'
  | 'evm_mint_adapter_missing'
  | 'evm_mint_network_mismatch'
  | 'stellar_mint_adapter_missing'
  | 'source_adapter_missing'
  | 'source_network_mismatch'
  | 'recipient_used_as_signer';

export interface WalletRoleMismatch {
  code: WalletRoleMismatchCode;
  role: string;
  message: string;
  expectedMasked?: string;
  currentMasked?: string;
}

export type WalletSigningIntent =
  | 'resume'
  | 'source_approval'
  | 'source_burn'
  | 'stellar_trustline'
  | 'stellar_mint'
  | 'evm_mint';

/** Underlying G-account for a Stellar G or M recipient (trustline signer). */
export function stellarRecipientTrustlineAccount(
  recipient: string,
): string | null {
  const trimmed = recipient.trim();
  if (StrKey.isValidEd25519PublicKey(trimmed)) return trimmed;
  if (StrKey.isValidMed25519PublicKey(trimmed)) {
    try {
      const raw = StrKey.decodeMed25519PublicKey(trimmed);
      // Muxed: 8-byte id + 32-byte ed25519 (SDK layout) or 32+8 depending on version.
      // stellar-base decodeMed25519PublicKey returns Buffer; encode the ed25519 key.
      const ed25519 =
        raw.length >= 40 ? raw.subarray(8, 40) : raw.subarray(0, 32);
      return StrKey.encodeEd25519PublicKey(ed25519);
    } catch {
      return null;
    }
  }
  return null;
}

export function normalizeEvmAddress(address: string): string | null {
  const trimmed = address.trim();
  if (!/^0x[a-fA-F0-9]{40}$/.test(trimmed)) return null;
  return trimmed.toLowerCase();
}

export function normalizeStellarGAddress(address: string): string | null {
  const trimmed = address.trim();
  if (!StrKey.isValidEd25519PublicKey(trimmed)) return null;
  return trimmed;
}

export function classifyStellarRecipient(
  address: string,
): StellarRecipientKind | null {
  const trimmed = address.trim();
  if (StrKey.isValidEd25519PublicKey(trimmed)) return 'stellar_g';
  if (StrKey.isValidMed25519PublicKey(trimmed)) return 'stellar_m';
  return null;
}

export function maskAddress(address: string): string {
  if (address.startsWith('0x') && address.length >= 10) {
    return `${address.slice(0, 6)}…${address.slice(-4)}`;
  }
  if (address.length > 12) {
    return `${address.slice(0, 5)}…${address.slice(-5)}`;
  }
  return address;
}

export function buildWalletRoleBindings(input: {
  direction: CctpDirection;
  sourceChainId: string;
  destChainId: string;
  sender?: string;
  recipient: string;
  mintSubmitter?: string;
}): CctpWalletRoleBindings | null {
  if (input.direction === 'evm_to_stellar') {
    const sender = input.sender ? normalizeEvmAddress(input.sender) : null;
    const recipientKind = classifyStellarRecipient(input.recipient);
    const submitter = input.mintSubmitter
      ? normalizeStellarGAddress(input.mintSubmitter)
      : null;
    if (!sender || !recipientKind || !submitter) return null;
    return {
      schemaVersion: WALLET_BINDINGS_SCHEMA_VERSION,
      direction: input.direction,
      recipient: { kind: recipientKind, address: input.recipient.trim() },
      sourceBurn: {
        address: sender,
        chainId: input.sourceChainId,
        adapterFamily: 'evm',
      },
      stellarMintSubmitter: { address: submitter },
    };
  }

  const sender = input.sender ? normalizeStellarGAddress(input.sender) : null;
  const recipient = input.recipient ? normalizeEvmAddress(input.recipient) : null;
  if (!sender || !recipient) return null;
  return {
    schemaVersion: WALLET_BINDINGS_SCHEMA_VERSION,
    direction: input.direction,
    recipient: { kind: 'evm', address: recipient },
    sourceBurn: {
      address: sender,
      chainId: input.sourceChainId,
      adapterFamily: 'stellar',
    },
    evmMintSubmitter: {
      mode: 'permissionless',
      chainId: input.destChainId,
    },
  };
}

function addressesEqualEvm(a: string, b: string): boolean {
  const na = normalizeEvmAddress(a);
  const nb = normalizeEvmAddress(b);
  return Boolean(na && nb && na === nb);
}

function addressesEqualStellarG(a: string, b: string): boolean {
  const na = normalizeStellarGAddress(a);
  const nb = normalizeStellarGAddress(b);
  return Boolean(na && nb && na === nb);
}

function mismatch(
  code: WalletRoleMismatchCode,
  role: string,
  message: string,
  expected?: string,
  current?: string,
): WalletRoleMismatch {
  return {
    code,
    role,
    message,
    expectedMasked: expected ? maskAddress(expected) : undefined,
    currentMasked: current ? maskAddress(current) : undefined,
  };
}

function payloadSourceAddress(payload: PreparedWalletPayload): string | undefined {
  return payload.source ?? payload.from;
}

function validatePayloadAgainstBindings(input: {
  bindings: CctpWalletRoleBindings;
  payload: PreparedWalletPayload;
  intent: WalletSigningIntent;
}): WalletRoleMismatch | null {
  const { bindings, payload, intent } = input;
  const needsSourceBurn =
    intent === 'resume' ||
    intent === 'source_approval' ||
    intent === 'source_burn';
  const needsStellarMint = intent === 'stellar_mint';
  const needsStellarTrustline = intent === 'stellar_trustline';
  const needsEvmMint = intent === 'evm_mint';

  if (payload.type === 'stellar_xdr') {
    if (payload.network_passphrase !== STELLAR_TESTNET_PASSPHRASE) {
      return mismatch(
        'source_network_mismatch',
        'source_burn',
        'Prepared Stellar network does not match this transfer.',
        bindings.sourceBurn.chainId,
      );
    }
    if (needsSourceBurn && bindings.sourceBurn.adapterFamily === 'stellar') {
      const source = payloadSourceAddress(payload);
      if (
        source &&
        !addressesEqualStellarG(source, bindings.sourceBurn.address)
      ) {
        return mismatch(
          'source_burn_mismatch',
          'source_burn',
          'Connect the original Stellar source wallet to continue.',
          bindings.sourceBurn.address,
          source,
        );
      }
    }
    if (needsStellarTrustline) {
      const expected =
        stellarRecipientTrustlineAccount(bindings.recipient.address) ??
        bindings.recipient.address;
      const source = payloadSourceAddress(payload);
      if (source && !addressesEqualStellarG(source, expected)) {
        return mismatch(
          'stellar_mint_submitter_mismatch',
          'stellar_trustline',
          'Connect Freighter as the USDC recipient G-account to open the trustline.',
          expected,
          source,
        );
      }
    }
    if (needsStellarMint && bindings.stellarMintSubmitter) {
      const source = payloadSourceAddress(payload);
      if (
        source &&
        !addressesEqualStellarG(source, bindings.stellarMintSubmitter.address)
      ) {
        return mismatch(
          'stellar_mint_submitter_mismatch',
          'stellar_mint_submitter',
          'Connect the original Stellar mint submitter (G) to continue.',
          bindings.stellarMintSubmitter.address,
          source,
        );
      }
    }
    return null;
  }

  if (needsEvmMint && bindings.evmMintSubmitter) {
    if (payload.chain_id !== bindings.evmMintSubmitter.chainId) {
      return mismatch(
        'evm_mint_network_mismatch',
        'evm_mint_submitter',
        'Prepared EVM mint network does not match this transfer.',
        bindings.evmMintSubmitter.chainId,
        payload.chain_id,
      );
    }
  }

  if (needsSourceBurn && bindings.sourceBurn.adapterFamily === 'evm') {
    if (payload.chain_id !== bindings.sourceBurn.chainId) {
      return mismatch(
        'source_network_mismatch',
        'source_burn',
        'Prepared EVM network does not match this transfer.',
        bindings.sourceBurn.chainId,
        payload.chain_id,
      );
    }
    const source = payloadSourceAddress(payload);
    if (source && !addressesEqualEvm(source, bindings.sourceBurn.address)) {
      return mismatch(
        'source_burn_mismatch',
        'source_burn',
        'Connect the original EVM source wallet to continue.',
        bindings.sourceBurn.address,
        source,
      );
    }
  }

  if (needsStellarMint || needsEvmMint) {
    return null;
  }

  return null;
}

export function assessWalletRoleBindings(input: {
  bindings: CctpWalletRoleBindings | undefined;
  wallets: CctpWalletRoles;
  intent: WalletSigningIntent;
  payload?: PreparedWalletPayload | null;
}): { ok: true } | { ok: false; issue: WalletRoleMismatch } {
  const { bindings, wallets, intent, payload } = input;
  if (!bindings) {
    return {
      ok: false,
      issue: mismatch(
        'bindings_missing',
        'session',
        'This saved transfer predates wallet verification. Start a new quote to continue.',
      ),
    };
  }

  if (payload) {
    const payloadIssue = validatePayloadAgainstBindings({
      bindings,
      payload,
      intent,
    });
    if (payloadIssue) {
      return { ok: false, issue: payloadIssue };
    }
  }

  if (bindings.recipient.kind === 'stellar_m') {
    const connectedMint =
      wallets.mintSubmitter ?? wallets.sourceAddress ?? '';
    if (
      connectedMint &&
      addressesEqualStellarG(connectedMint, bindings.recipient.address)
    ) {
      return {
        ok: false,
        issue: mismatch(
          'recipient_used_as_signer',
          'recipient',
          'Muxed (M) recipients cannot sign. Connect the Stellar G mint submitter wallet.',
          bindings.stellarMintSubmitter?.address,
          connectedMint,
        ),
      };
    }
  }

  const needsSourceBurn =
    intent === 'resume' ||
    intent === 'source_approval' ||
    intent === 'source_burn';
  const needsStellarMint = intent === 'stellar_mint';
  const needsStellarTrustline = intent === 'stellar_trustline';
  const needsEvmMint = intent === 'evm_mint';

  if (needsSourceBurn) {
    if (bindings.sourceBurn.adapterFamily === 'evm') {
      if (!wallets.sourceEvmAdapterId) {
        return {
          ok: false,
          issue: mismatch(
            'source_adapter_missing',
            'source_burn',
            'Connect the original EVM source wallet to continue.',
            bindings.sourceBurn.address,
            wallets.sourceAddress,
          ),
        };
      }
      if (!wallets.sourceAddress) {
        return {
          ok: false,
          issue: mismatch(
            'source_burn_mismatch',
            'source_burn',
            'Connect the original EVM source wallet to continue.',
            bindings.sourceBurn.address,
          ),
        };
      }
      if (!addressesEqualEvm(wallets.sourceAddress, bindings.sourceBurn.address)) {
        return {
          ok: false,
          issue: mismatch(
            'source_burn_mismatch',
            'source_burn',
            'Connect the original EVM source wallet to continue.',
            bindings.sourceBurn.address,
            wallets.sourceAddress,
          ),
        };
      }
    } else {
      if (!wallets.sourceStellarAdapterId) {
        return {
          ok: false,
          issue: mismatch(
            'source_adapter_missing',
            'source_burn',
            'Connect the original Stellar source wallet to continue.',
            bindings.sourceBurn.address,
            wallets.sourceAddress,
          ),
        };
      }
      if (
        !wallets.sourceAddress ||
        !addressesEqualStellarG(wallets.sourceAddress, bindings.sourceBurn.address)
      ) {
        return {
          ok: false,
          issue: mismatch(
            'source_burn_mismatch',
            'source_burn',
            'Connect the original Stellar source wallet to continue.',
            bindings.sourceBurn.address,
            wallets.sourceAddress,
          ),
        };
      }
    }
  }

  if (needsStellarTrustline) {
    const expectedG =
      stellarRecipientTrustlineAccount(bindings.recipient.address) ?? '';
    if (!wallets.mintSubmitterStellarAdapterId && !wallets.sourceStellarAdapterId) {
      return {
        ok: false,
        issue: mismatch(
          'stellar_mint_adapter_missing',
          'stellar_trustline',
          'Connect Freighter as the USDC recipient G-account to open the trustline.',
          expectedG || undefined,
        ),
      };
    }
    const connected = wallets.mintSubmitter ?? wallets.sourceAddress ?? '';
    if (!expectedG || !connected || !addressesEqualStellarG(connected, expectedG)) {
      return {
        ok: false,
        issue: mismatch(
          'stellar_mint_submitter_mismatch',
          'stellar_trustline',
          'Connect Freighter as the USDC recipient G-account to open the trustline.',
          expectedG || undefined,
          connected || undefined,
        ),
      };
    }
  }

  if (needsStellarMint && bindings.stellarMintSubmitter) {
    if (!wallets.mintSubmitterStellarAdapterId && !wallets.sourceStellarAdapterId) {
      return {
        ok: false,
        issue: mismatch(
          'stellar_mint_adapter_missing',
          'stellar_mint_submitter',
          'Connect the original Stellar mint submitter (G) to continue.',
          bindings.stellarMintSubmitter.address,
          wallets.mintSubmitter,
        ),
      };
    }
    const connected =
      wallets.mintSubmitter ?? wallets.sourceAddress ?? '';
    if (
      !connected ||
      !addressesEqualStellarG(connected, bindings.stellarMintSubmitter.address)
    ) {
      return {
        ok: false,
        issue: mismatch(
          'stellar_mint_submitter_mismatch',
          'stellar_mint_submitter',
          'Connect the original Stellar mint submitter (G) to continue.',
          bindings.stellarMintSubmitter.address,
          connected || undefined,
        ),
      };
    }
  }

  if (needsEvmMint && bindings.evmMintSubmitter) {
    if (bindings.evmMintSubmitter.mode !== 'permissionless') {
      return {
        ok: false,
        issue: mismatch(
          'evm_mint_adapter_missing',
          'evm_mint_submitter',
          'Connect an EVM wallet on the destination chain to submit the mint.',
        ),
      };
    }
    if (!wallets.evmDestinationAdapterId) {
      return {
        ok: false,
        issue: mismatch(
          'evm_mint_adapter_missing',
          'evm_mint_submitter',
          'Connect an EVM wallet on the destination chain to submit the mint.',
        ),
      };
    }
  }

  return { ok: true };
}

export function signingIntentForBurnStep(
  step: 'approval_ready' | 'burn_ready',
): WalletSigningIntent {
  return step === 'approval_ready' ? 'source_approval' : 'source_burn';
}

export function signingIntentForMintPayload(
  payload: PreparedWalletPayload,
  trustlineRequired?: boolean,
): WalletSigningIntent {
  if (trustlineRequired && payload.type === 'stellar_xdr') {
    return 'stellar_trustline';
  }
  return payload.type === 'stellar_xdr' ? 'stellar_mint' : 'evm_mint';
}

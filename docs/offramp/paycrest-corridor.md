# Paycrest offramp corridor

Paycrest does not use Stellar as a deposit network. Its supported deposit networks are EVM, Starknet, and Tron, and Paycrest institution identifiers are eight-character codes; those codes are not CBN bank codes.

Stellar users should bridge or settle their funds into a supported Paycrest deposit network before starting an offramp. A Stellar asset should therefore not be presented as a direct Paycrest deposit inside `/swap`.

This note is documentation-only. It does not change swap preparation, quote selection, wallet signing, API contracts, CORS settings, or any production feature flag.

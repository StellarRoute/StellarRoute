# Security Assumptions

## Runtime Assumptions

1. **Soroban runtime is correct**: We assume the Soroban VM executes contract code faithfully, enforces storage isolation, and correctly manages authentication via `require_auth()`.

2. **Single-threaded execution**: Each contract invocation is processed atomically. No concurrent state mutations within a single invocation.

3. **Storage integrity**: Instance and persistent storage behave as documented. Data persists as long as TTLs are maintained.

4. **Cryptographic primitives**: Address verification, signatures, and hashing provided by the Soroban SDK are secure.

## Operational Assumptions

5. **Admin key security**: The admin private key is stored securely (hardware wallet or encrypted vault). Compromise of the admin key allows full contract control.

6. **Pool vetting**: Only vetted, audited liquidity pool contracts are registered. The router trusts registered pools to return honest swap outputs.

7. **TTL maintenance**: Operational scripts or automated processes will extend storage TTLs before expiry. If TTLs expire, contract state is lost and redeployment is required. Operators should configure alerting for TTL failures (for example via `TTL_ALERT_WEBHOOK_URL`) and escalate if the extension job fails twice in a row.

8. **Network availability**: The Stellar network and Soroban RPC endpoints are available for contract interactions.

## Trust Boundaries

| Boundary | Trusted Side | Untrusted Side |
|----------|-------------|----------------|
| Admin authentication | Soroban `require_auth()` | External callers |
| Pool contract calls | Router invocation logic | Pool return values |
| Storage persistence | Soroban storage layer | TTL expiry (time-based) |
| Fee calculation | Contract arithmetic | User-supplied route data |

## What We Do NOT Assume

- Pool contracts are bug-free (we handle call failures gracefully).
- Users provide optimal routes (the contract validates but does not optimize).
- The admin acts honestly (governance is out of scope for the contract layer).
- Gas/resource costs remain stable (fee parameters may need adjustment).

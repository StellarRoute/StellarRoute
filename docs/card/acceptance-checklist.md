# Card preview acceptance checklist

This checklist closes the CARD-01 through CARD-49 preview sequence with a
repeatable **sandbox-fixture** demonstration. It is not a production enablement
procedure and does not authorize live card traffic.

> Use only a sandbox partner, synthetic fixtures, and test tokens. Never enter
> or store a real PAN, CVV, track value, private key, webhook secret, or live
> issuer credential. StellarRoute must never hold card PANs or partner keys.

## Preconditions

- [ ] The reviewer has a disposable sandbox environment and fixture bundle.
- [ ] `CARD_ENABLED` is unset/false everywhere by default.
- [ ] The preview API/frontend is explicitly enabled only in the sandbox.
- [ ] `NEXT_PUBLIC_CARD_PARTNER_STELLAR_ADDRESS` contains the sandbox partner
      G-address, or the expected `partner_unconfigured` state is recorded.
- [ ] `CARD_WEBHOOK_HMAC_KEY` is generated and stored in the host secret store;
      it is not committed, logged, or exposed as a public variable.
- [ ] The reviewer knows which fixture is being replayed and has verified that
      it contains no live identifiers.
- [ ] The existing swap, quote, CCTP, and offramp configurations are recorded
      but not changed for this demo.

## Coverage index: CARD-01 through CARD-49

Use this index as the review checklist. Mark an item only after its focused
fixture or contract test has passed; this document does not replace the
implementation tests.

- [ ] CARD-01
- [ ] CARD-02
- [ ] CARD-03
- [ ] CARD-04
- [ ] CARD-05
- [ ] CARD-06
- [ ] CARD-07
- [ ] CARD-08
- [ ] CARD-09
- [ ] CARD-10
- [ ] CARD-11
- [ ] CARD-12
- [ ] CARD-13
- [ ] CARD-14
- [ ] CARD-15
- [ ] CARD-16
- [ ] CARD-17
- [ ] CARD-18
- [ ] CARD-19
- [ ] CARD-20
- [ ] CARD-21
- [ ] CARD-22
- [ ] CARD-23
- [ ] CARD-24
- [ ] CARD-25
- [ ] CARD-26
- [ ] CARD-27
- [ ] CARD-28
- [ ] CARD-29
- [ ] CARD-30
- [ ] CARD-31
- [ ] CARD-32
- [ ] CARD-33
- [ ] CARD-34
- [ ] CARD-35
- [ ] CARD-36
- [ ] CARD-37
- [ ] CARD-38
- [ ] CARD-39
- [ ] CARD-40
- [ ] CARD-41
- [ ] CARD-42
- [ ] CARD-43
- [ ] CARD-44
- [ ] CARD-45
- [ ] CARD-46
- [ ] CARD-47
- [ ] CARD-48
- [ ] CARD-49

## Fixture demo

Record timestamps, fixture revision, and pass/fail results for each step. A
failed step stops the demo; do not work around it with a live issuer.

1. **Open the flagged preview** — Open `/card` in the sandbox with the card
   public flag enabled. Confirm the KYC acknowledgement is visible and that the
   partner state is either the configured sandbox partner or the explicit
   `partner_unconfigured` response.
2. **Preview a EUR charge** — Use the synthetic application fixture to preview
   a EUR charge against the sandbox USDC balance. Confirm currency, amount,
   partner destination, and fee/authorization summary. Do not sign, broadcast,
   or submit a live payment.
3. **Cancel safely** — Cancel before authorization. Confirm the application is
   discarded or returned to the documented pre-authorization state and that no
   hold, clearing, or partner event is created.
4. **Replay the hold** — Replay the approved sandbox authorization fixture.
   Confirm the expected hold is reflected in the card ledger/reconciliation
   output and that the partner event identifier is recorded.
5. **Replay the clearing** — Replay the matching sandbox clearing fixture.
   Confirm held funds move to spent exactly once, the event is idempotent when
   replayed, and the reconciliation result is balanced.
6. **Review the audit trail** — Confirm only redacted, non-sensitive metadata is
   retained. Verify that no PAN, CVV, track value, HMAC key, or raw secret is
   present in logs or artifacts.

## Non-goals and frozen paths

This demo does **not**:

- use a live BIN, issuer, card program, or real cardholder data;
- store or transmit a PAN, CVV, track value, or private key;
- change classic one-hop SDEX prepare → wallet sign → submit;
- change quote selection/ranking or its wire contract;
- change wallet connect/sign adapters;
- change `/swap`, `/offramp`, or `/cross-chain-swap` layout;
- change existing OpenAPI field names, types, error codes, or route behavior;
- change CORS allowlists, the `CCTP_ENABLED` default, `real_xdr` pinning, or
  offramp payout behavior;
- enable a production flag or modify an EC2/Vercel deployment workflow.

## Review sign-off

- [ ] All CARD-01 through CARD-49 items are either verified or explicitly
      marked not applicable with a reason.
- [ ] The fixture demo completed in order: KYC acknowledgement → EUR/USDC
      preview → cancel → hold → clearing.
- [ ] Reconciliation reported no unexplained held/spent mismatch.
- [ ] Existing swap/quote/OpenAPI contract tests passed unchanged.
- [ ] The reviewer recorded the fixture revision and evidence location without
      recording secrets or sensitive card data.
- [ ] Preview flags were disabled and sandbox credentials were revoked or
      rotated after the review.

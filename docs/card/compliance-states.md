# Card compliance states

This document maps card application and account states to the existing KYC-CDD
and AML-CFT-CPF compliance packs. These controls are cumulative: an AML check
does not replace customer due diligence, and a completed KYC review does not
waive ongoing AML monitoring.

## Policy sources

- **KYC-CDD** (`docs/compliance/`): customer identification and verification,
  customer due diligence, enhanced due diligence when required, and ongoing
  review.
- **AML-CFT-CPF** (`docs/compliance/`): sanctions and other required screening,
  risk assessment, ongoing monitoring, escalation, and applicable restrictions
  or reporting.

The compliance pack files are not present in this checkout, so the mappings
below identify policy topics rather than asserting unverified section numbers.
Before production use, reconcile each topic with the exact section identifiers
in the canonical KYC-CDD and AML-CFT-CPF documents. This state map does not
weaken or replace either policy.

## States

| State | Meaning | Applicable policy sections/topics | Transition requirement |
|---|---|---|---|
| `kyc_required` | An application exists, but required identity information or verification is incomplete. | KYC-CDD: customer identification, identity verification, and customer due diligence. AML-CFT-CPF: required initial screening and risk assessment. | Remain here until the required due diligence and screening inputs have been collected and review can begin. This state is not approval to issue or activate a card. |
| `pending` | Required information has been submitted and is under review, or a required check remains unresolved. | KYC-CDD: completion and review of customer due diligence, including enhanced due diligence when applicable. AML-CFT-CPF: screening disposition, risk assessment, and escalation of unresolved matches or alerts. | Remain pending while any required check, escalation, or decision is outstanding. Do not issue or activate a card while pending. |
| `active` | The card account is approved for the permitted program activity. | KYC-CDD: completed and approved customer due diligence, with any required enhanced due diligence. AML-CFT-CPF: required screening resolved and risk decision approved, with ongoing monitoring in force. | **Active is unreachable unless KYC-CDD is complete and approved and required AML-CFT-CPF checks are resolved.** Any required escalation or unresolved review keeps the account out of this state. |
| `frozen` | Activity is restricted while a compliance concern, review, legal restriction, or other authorized hold is handled. | KYC-CDD: ongoing review and any required refresh or enhanced due diligence. AML-CFT-CPF: ongoing monitoring, alert investigation, escalation, and applicable restriction or reporting procedures. | Apply the restrictions required by the applicable policy and authorized decision. Return to `active` only after required reviews are resolved and approval is recorded; otherwise proceed to `canceled` where policy requires. |
| `canceled` | The card account is closed and must not accept new activity. | KYC-CDD: applicable customer-file retention and review obligations. AML-CFT-CPF: applicable record retention, investigation, escalation, and reporting obligations. | No return to `active`. Preserve or dispose of records only according to applicable policy and legal requirements; cancellation does not terminate outstanding compliance obligations. |

## Sensitive card data

StellarRoute must never store or log a card PAN, CVV, or magstripe/track data.
Do not add these values to application records, logs, analytics, fixtures, or
support artifacts. Card data handling, if any, belongs with an appropriately
authorized payment partner and its compliant environment, not StellarRoute.

## Scope and rollout

This document defines policy mapping only. It does not define document-upload
formats, collect identity documents, or authorize a production card flow. It
does not change application behavior, API contracts, feature flags, or any live
swap, quote, offramp, or cross-chain-swap path. Card routes remain unavailable
when their feature flag is unset or false.
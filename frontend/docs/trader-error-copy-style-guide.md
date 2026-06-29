# Trader-Facing Error Copy and Tone Style Guide

Issue: #464  
Linked frontend mapping work: #310

## Objective
Create consistent, trader-facing error copy for quote, wallet, and network failures.

## Voice and Tone Principles
- Be direct and calm: describe what happened without blame.
- Focus on next action: every message should include a recovery step.
- Stay specific: mention the affected task (quote refresh, wallet confirmation, pair selection).
- Keep confidence: avoid catastrophic language and avoid exposing backend internals.
- Keep it short: headline first, explanation second, recovery action third.

## Message Pattern (Required)
Every trader-facing error should follow this structure:
- Headline: short state statement.
- Explanation: one sentence with context.
- Recovery action: explicit next step.

Template:
- Headline: What happened now.
- Explanation: Why this blocked the action.
- Recovery action: What the trader can do next.

## Tone Rules
- Use neutral wording: "could not", "not available", "temporarily limited".
- Do not use blame wording: "you entered wrong", "invalid user", "you failed".
- Do not use panic wording: "fatal", "catastrophic", "critical failure".
- Use action verbs in CTAs: "Refresh quote", "Reconnect wallet", "Adjust trade size".

## Canonical Error Mapping (Frontend)
The shared copy mapping is centralized in:
- frontend/lib/api/trader-error-copy.ts

The mapper accepts `StellarRouteApiError` codes from:
- frontend/lib/api/client.ts

Mapped API codes:
- validation_error
- invalid_asset
- no_route
- stale_market_data
- rate_limit_exceeded
- overloaded
- bad_request
- unauthorized
- not_found
- internal_error
- network_error
- unknown_error (safe fallback)

## Example Library (12+)
These examples are canonical samples for design review and QA.

1. validation_error
- Headline: Check your trade details
- Explanation: One or more inputs are outside the allowed format or range.
- Recovery action: Update the amount or pair, then refresh the quote.

2. invalid_asset
- Headline: This asset pair is not available right now
- Explanation: The selected asset format or issuer could not be matched.
- Recovery action: Choose a supported asset pair and try again.

3. no_route
- Headline: No executable route found
- Explanation: Current liquidity cannot complete this trade at the requested size.
- Recovery action: Try a smaller amount or a different pair.

4. stale_market_data
- Headline: Market data is still updating
- Explanation: Fresh pricing is not available yet for this route.
- Recovery action: Wait a moment and refresh to fetch a current quote.

5. rate_limit_exceeded
- Headline: Quote refresh is temporarily limited
- Explanation: Too many quote requests were sent in a short window.
- Recovery action: Wait briefly before refreshing again.

6. overloaded
- Headline: Quote service is handling high traffic
- Explanation: Routing services are taking longer than normal to respond.
- Recovery action: Retry in a moment to request a fresh quote.

7. bad_request
- Headline: We could not process this request
- Explanation: The quote request did not match the expected API format.
- Recovery action: Refresh and try again with updated trade inputs.

8. unauthorized
- Headline: Session check required
- Explanation: Your current request needs a valid session context.
- Recovery action: Reconnect wallet or reload the page, then retry.

9. not_found
- Headline: Requested market data was not found
- Explanation: The selected pair or route data is currently unavailable.
- Recovery action: Pick another pair and request a new quote.

10. internal_error
- Headline: Quote service hit an internal issue
- Explanation: The request reached the server but could not be completed safely.
- Recovery action: Retry shortly while we stabilize the route response.

11. network_error
- Headline: Network connection interrupted
- Explanation: The app could not reach routing services from this device.
- Recovery action: Check your connection and refresh once online.

12. wallet_rejected (inferred from generic wallet errors)
- Headline: Wallet action was not completed
- Explanation: The wallet did not confirm the request needed to continue.
- Recovery action: Reopen your wallet, approve the request, and submit again.

13. unknown_error
- Headline: We could not refresh this quote
- Explanation: Something unexpected happened while preparing your trade details.
- Recovery action: Refresh the quote, then try again.

## UI Integration Notes
Current trader-facing usages now route through shared mapping in:
- frontend/components/swap/SwapCard.tsx
- frontend/components/DemoSwap.tsx

Integration contract:
- Convert raw errors to canonical copy using `getTraderErrorCopy(error)`.
- Render the message as headline plus recovery action for compact UI blocks.
- Keep CTA labels available for future buttonized recovery patterns.

## QA Checklist
- Message includes headline, explanation, and recovery action.
- Copy avoids blame language.
- Unknown errors use safe fallback text.
- Quote and wallet failure surfaces use shared mapper, not raw `error.message`.
- Recovery action is actionable and specific.

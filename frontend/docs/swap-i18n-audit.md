# Swap and error i18n audit notes

## Fallback locale

- Swap UI translations use `SWAP_FALLBACK_LOCALE` in `frontend/lib/swap-i18n.ts`.
- Current fallback locale: `en-US`.

## Audit checklist

- [x] Swap CTA labels use translation keys.
- [x] Swap retry/recovery banners use translation keys.
- [x] Swap pricing panel labels/tooltips use translation keys.
- [x] Keyboard shortcut help content uses translation keys.
- [x] Avoid exposing raw backend error codes without a user-facing message.
- [x] TransactionConfirmationModal — all user-visible strings routed through `useSwapI18n`.

## TransactionConfirmationModal i18n coverage

All 31 `swap.confirm.*` keys are defined for **en-US**, **zh-CN**, and **es-ES**.
Other locales fall back to `en-US` via `SWAP_LOCALE_ALIASES`.

| Namespace | Keys |
|---|---|
| `swap.confirm.review.*` | `heading`, `description`, `announcement` |
| `swap.confirm.pending.*` | `heading`, `description`, `announcement` |
| `swap.confirm.submitted.*` | `heading`, `description`, `announcement` |
| `swap.confirm.confirmed.*` | `heading`, `description`, `announcement` |
| `swap.confirm.failed.*` | `heading`, `description`, `announcement` |
| `swap.confirm.dropped.*` | `heading`, `description`, `announcement` |
| `swap.confirm.summary.*` | `youPay`, `youReceive`, `minReceived` |
| `swap.confirm.cta.*` | `confirmSwap`, `cancel`, `processing`, `done`, `tryAgain`, `dismiss`, `resubmit` |

### Test strategy

The test suite (`TransactionConfirmationModal.test.tsx`) uses a **key-echo mock** for
`useSwapI18n` — `t(key)` returns `[key]` — so every `expect` assertion confirms the
component resolves strings via the hook rather than embedding hardcoded English.
Locale correctness (en-US values, zh-CN values) is verified directly against
`createSwapTranslator` without a DOM render.

## Quick coverage command

Run this check to find direct quoted strings in swap components during review:

```bash
rg -n '"[A-Za-z][^"]+"' frontend/components/swap frontend/app/swap
```

Use this output as an audit aid and verify any user-visible string is mapped through `useSwapI18n()`.

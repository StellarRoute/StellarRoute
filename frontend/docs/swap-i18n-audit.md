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

## Quick coverage command

Run this check to find direct quoted strings in swap components during review:

```bash
rg -n '"[A-Za-z][^"]+"' frontend/components/swap frontend/app/swap
```

Use this output as an audit aid and verify any user-visible string is mapped through `useSwapI18n()`.


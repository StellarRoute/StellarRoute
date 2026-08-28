# Feature Flags

StellarRoute uses a lightweight, two-layer feature flag system to gate experimental UI features (routes beta, swap UI v2, etc.) without requiring a redeploy.

## How it works

Ordinary flags are resolved in this priority order **after hydration** (remote fetch and window overrides applied):

| Priority | Source | How |
|---|---|---|
| 1 (highest) | Remote config | JSON file fetched from `NEXT_PUBLIC_FLAGS_URL` |
| 2 | Window override | `window.__STELLAR_ROUTE_FLAGS__ = { flag_name: true }` (dev/e2e only; applied after mount) |
| 3 | Environment variable | `NEXT_PUBLIC_FLAG_<NAME>=true` |
| 4 (default) | Hardcoded | Always `false` (default-off) |

### SSR / hydration (initial render)

The server and the first client paint use an **SSR-safe snapshot** that never reads `window.__STELLAR_ROUTE_FLAGS__`:

| Priority | Source |
|---|---|
| 1 | Security-pinned env/default (`real_xdr`) |
| 2 | Warmed remote cache (module cache only when already fetched) |
| 3 | Environment variable |
| 4 (default) | `false` |

Window overrides apply in a `useEffect` after mount. Remote fetch then applies full post-hydration precedence (`remote > window > env`). This avoids hydration mismatches while still supporting dev/e2e `window.__STELLAR_ROUTE_FLAGS__` toggles.

When only `NEXT_PUBLIC_FLAGS_URL` is set (no env), hooks start in `loading: true` with `enabled: false` until the remote JSON resolves.

Ordinary flags are **off by default**. You must explicitly enable them.

### Security-pinned flags (`real_xdr`)

`real_xdr` is **not** remotely killable. Precedence:

| Priority | Source | How |
|---|---|---|
| 1 | Environment variable | `NEXT_PUBLIC_FLAG_REAL_XDR=true\|false` |
| 2 (default) | Hardcoded | `true` (API prepare → wallet sign → API submit) |

Remote `FLAGS_URL` values for `real_xdr` are **ignored**. A remote `{"real_xdr": false}` cannot turn off the only secure swap execution path when env is unset (default on) or explicitly `true`.

Operational kill switches belong on the **backend** (provider kill-switch / dependency health), not in client feature flags and not via fallback client-built XDR.

While ordinary flags are still loading, swap execution stays **fail-closed** and API-only — never an alternate client-XDR path.

---

## Adding a new flag

**1. Register the flag name** in `hooks/useFeatureFlag.ts`:

```ts
export type FlagName =
  | "routes_beta"
  | "batch_swaps"
  | "swap_ui_v2"
  | "your_new_flag";  // ← add here using snake_case
```

**2. Enable via env** (local dev / Vercel preview):

```bash
NEXT_PUBLIC_FLAG_YOUR_NEW_FLAG=true
```

**3. Enable via remote config** (production, no redeploy needed):

Deploy or update your flags JSON file at `NEXT_PUBLIC_FLAGS_URL`:

```json
{
  "routes_beta": true,
  "batch_swaps": true,
  "your_new_flag": false
}
```

Do **not** put `real_xdr` in remote JSON expecting it to disable live swaps — it will be ignored.

---

## Using flags in components

**Single flag:**

```tsx
import { useFeatureFlag } from "@/hooks/useFeatureFlag";

export function MyComponent() {
  const { enabled, loading } = useFeatureFlag("routes_beta");
  if (loading) return null;
  return enabled ? <NewUI /> : <LegacyUI />;
}
```

**Multiple flags at once:**

```tsx
import { useFeatureFlags } from "@/hooks/useFeatureFlag";

export function SwapPage() {
  const flags = useFeatureFlags(["routes_beta", "swap_ui_v2"]);
  return (
    <>
      {flags.routes_beta && <RoutesBeta />}
      {flags.swap_ui_v2 && <SwapV2 />}
    </>
  );
}
```

---

## Environment variables

| Variable | Description |
|---|---|
| `NEXT_PUBLIC_FLAGS_URL` | URL to remote JSON flags config (optional; does **not** control `real_xdr`) |
| `NEXT_PUBLIC_FLAG_ROUTES_BETA` | Enable routes beta (`true`/`false`) |
| `NEXT_PUBLIC_FLAG_BATCH_SWAPS` | Enable batch swaps (`true`/`false`) |
| `NEXT_PUBLIC_FLAG_SWAP_UI_V2` | Enable swap UI v2 cross-chain route deck (`true`/`false`). When on, `/swap` renders the wide Stellar-centered corridor UI; when off, the legacy swap card experience is unchanged. |
| `NEXT_PUBLIC_FLAG_TRANSACTION_HISTORY` | Enable transaction history filters and controls on `/history` (`true`/`false`) |
| `NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE` | Enable advanced slippage controls |
| `NEXT_PUBLIC_FLAG_REAL_XDR` | **Default on** when unset. Classic API prepare → Freighter sign → API submit → Horizon confirm (`real_xdr`; one-hop SDEX only). Security-pinned: remote flags cannot disable. When false, product swaps fail closed (no client-XDR fallback). |

---

## Cleaning up flags

Once a feature is stable and fully rolled out:

1. Remove the `FlagName` entry from `hooks/useFeatureFlag.ts`
2. Remove the `useFeatureFlag` call from the component
3. Delete the gate wrapper if one was created
4. Remove the env var from `.env` files and CI config

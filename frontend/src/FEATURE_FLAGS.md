# Feature Flags

StellarRoute uses a lightweight, two-layer feature flag system to gate experimental UI features (routes beta, swap UI v2, etc.) without requiring a redeploy.

## How it works

Flags are resolved in this priority order:

| Priority | Source | How |
|---|---|---|
| 1 (highest) | Remote config | JSON file fetched from `NEXT_PUBLIC_FLAGS_URL` |
| 2 | Environment variable | `NEXT_PUBLIC_FLAG_<NAME>=true` |
| 3 (default) | Hardcoded | Always `false` (default-off) |

All flags are **off by default**. You must explicitly enable them.

---

## Adding a new flag

**1. Register the flag name** in `useFeatureFlag.ts`:

```ts
export type FlagName =
  | "routes_beta"
  | "swap_ui_v2"
  | "your_new_flag";  // ← add here
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
  "your_new_flag": false
}
```

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

**Gate wrapper component** (cleanest pattern for page-level gates):

```tsx
import { RoutesBetaGate } from "@/components/RoutesBetaGate";

export default function SwapPage() {
  return (
    <RoutesBetaGate fallback={<LegacyRoutes />}>
      <RoutesBeta />
    </RoutesBetaGate>
  );
}
```

---

## Environment variables

| Variable | Description |
|---|---|
| `NEXT_PUBLIC_FLAGS_URL` | URL to remote JSON flags config (optional) |
| `NEXT_PUBLIC_FLAG_ROUTES_BETA` | Enable routes beta (`true`/`false`) |
| `NEXT_PUBLIC_FLAG_SWAP_UI_V2` | Enable swap UI v2 (`true`/`false`) |
| `NEXT_PUBLIC_FLAG_TRANSACTION_HISTORY` | Enable transaction history tab |
| `NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE` | Enable advanced slippage controls |

---

## Cleaning up flags

Once a feature is stable and fully rolled out:

1. Remove the `FlagName` entry from `useFeatureFlag.ts`
2. Remove the `useFeatureFlag` call from the component
3. Delete the gate wrapper if one was created
4. Remove the env var from `.env` files and CI config

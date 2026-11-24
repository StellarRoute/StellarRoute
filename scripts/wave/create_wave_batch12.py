#!/usr/bin/env python3
"""Create Wave 12 Drip issues on StellarRoute/StellarRoute (run from repo root)."""
from __future__ import annotations

import json
import subprocess
import sys
import time
from textwrap import dedent

REPO = "StellarRoute/StellarRoute"

ADDITIVE = dedent(
    """
    ## Additive constraint
    This issue is additive-only. A merged PR must not change anything that already works for live users.

    **Frozen (do not modify behavior):**
    - Classic one-hop SDEX prepare → wallet sign → submit (`crates/api/src/routes/swap.rs`)
    - Quote selection / ranking (`crates/api/src/routes/quote.rs` ranking path)
    - Wallet connect/sign adapters used by the live swap CTA
    - Existing OpenAPI field names/types and error codes
    - CORS allowlists, `CCTP_ENABLED` default, `real_xdr` pinning
    - `/swap`, `/offramp`, and `/cross-chain-swap` layout unless a new flag defaults **false** and is unset in production

    **Forbidden:** renaming/removing JSON fields; tightening lean CI; enabling flags in Vercel or `.env.example`.

    New routes return 404 when their flag is unset or false. StellarRoute never holds keys or card PANs.
    """
).strip()


def card_body(seq: int, title: str, why: str, allowed: str, scope: str, acceptance: str, deps: str = "") -> str:
    seq_line = f"Sequence\nCARD-{seq} of AI-01 → AI-50, then CARD-01 → CARD-65."
    if deps:
        seq_line += f"\nDepends on: {deps}"
    return "\n\n".join(
        [
            seq_line,
            f"## Why\n{why}",
            ADDITIVE,
            f"## Allowed files\n{allowed}",
            f"## Scope\n{scope}",
            f"## Acceptance criteria\n{acceptance}",
        ]
    )


def fix_body(title: str, why: str, allowed: str, scope: str, acceptance: str) -> str:
    frozen = dedent(
        """
        ## Change constraint
        Targeted fix only — change behavior named in Scope; frozen surfaces stay frozen (swap prepare/submit, quote ranking, OpenAPI codes, CCTP default off).
        """
    ).strip()
    return "\n\n".join(
        [
            f"## Why\n{why}",
            frozen,
            f"## Allowed files\n{allowed}",
            f"## Scope\n{scope}",
            f"## Acceptance criteria\n{acceptance}",
        ]
    )


# (label prefix, title suffix, labels list, body)
ISSUES: list[tuple[str, str, list[str], str]] = []

CARD_TOPICS: list[tuple[int, str, str, str, str, str, str]] = [
    (
        57,
        "Card deck keyboard shortcut to freeze/unfreeze",
        "Power users need a fast freeze toggle without hunting the menu.",
        "frontend/lib/card/freeze-shortcut.ts, frontend/components/card/*, tests",
        "Register a `window` keydown handler (when card page focused) for `Shift+F` that calls existing freeze action behind `CARD_ENABLED`.",
        "- Shortcut works only when card flag on and wallet connected\n- Vitest covers handler guardrails\n- No change when flag off",
        "CARD-19",
    ),
    (
        58,
        "Print-friendly card activity statement layout",
        "Users export activity; print CSS should hide chrome and show tabular debits.",
        "frontend/app/card/print.css or module, CardActivity*.tsx, tests",
        "Add `@media print` styles hiding nav and showing monochrome activity table.",
        "- Print preview hides wallet chrome\n- Existing screen layout unchanged\n- Vitest snapshot or DOM test for print class",
        "CARD-51",
    ),
    (
        59,
        "Card activity full-text search across merchant memo",
        "Filtering by merchant alone misses memo keywords.",
        "frontend/lib/card/activity-search.ts, components, tests",
        "Client-side search index over activity fixtures; debounced input.",
        "- Search is case-insensitive\n- Empty query shows full list\n- Vitest for tokenization",
        "CARD-53",
    ),
    (
        60,
        "Aria-live announcements for card balance changes",
        "Screen-reader users miss balance updates after top-up.",
        "frontend/components/card/CardBalanceAnnouncer.tsx, tests",
        "Polite `aria-live` region when USDC balance string changes.",
        "- Only fires on real delta\n- Respects reduced-motion\n- Vitest render test",
        "CARD-19",
    ),
    (
        61,
        "Remember last selected fiat currency in localStorage",
        "FX module should persist currency pick per G-address.",
        "frontend/lib/card/fx-preferences.ts, FX components, tests",
        "Key `stellarroute_card_fx_ccy:<G>` with validation against supported set.",
        "- Invalid stored value falls back to USD\n- Switching wallet switches currency\n- Vitest isolation",
        "CARD-54",
    ),
    (
        62,
        "Card top-up amount presets (25/50/100 USDC)",
        "One-tap presets speed common top-up sizes.",
        "frontend/components/card/CardTopUpPresets.tsx, tests",
        "Chip row sets amount field; disabled when frozen.",
        "- Presets configurable constant\n- Vitest click handlers\n- No API calls in component",
        "CARD-45",
    ),
    (
        63,
        "Deep link /card?highlight=activity opens feed scrolled",
        "Support email links into activity section.",
        "frontend/app/card/CardPageClient.tsx, tests",
        "Read `highlight` search param; scroll activity into view once.",
        "- Invalid param ignored\n- No layout change without param\n- Vitest for parser",
        "CARD-44",
    ),
    (
        64,
        "Card partner misconfiguration inline help panel",
        "When partner URL missing, show actionable setup doc link (sandbox only).",
        "frontend/components/card/CardPartnerHelp.tsx, tests",
        "Render when `partner_unconfigured` health state from fixtures.",
        "- No secrets in copy\n- Hidden when partner configured\n- Vitest states",
        "CARD-20",
    ),
    (
        65,
        "CARD-65 capstone: card preview E2E smoke in CI",
        "Close CARD sequence with Playwright smoke behind card flag in CI job only.",
        "frontend/e2e/card-capstone.spec.ts, workflow snippet docs",
        "Spec mirrors CARD-40 patterns; document env in preview-enablement.md.",
        "- Job is optional/skipped without secrets\n- Additive workflow only\n- Playwright passes locally with mocks",
        "CARD-40",
    ),
]

for seq, short, why, allowed, scope, ac, deps in CARD_TOPICS:
    title = f"[card] CARD-{seq} {short}"
    body = card_body(seq, short, why, allowed, scope, ac, deps)
    ISSUES.append(
        (
            "card",
            title,
            ["enhancement", "help wanted", "Stellar Wave", "frontend", "complexity: medium"]
            if seq not in (57, 62, 63)
            else ["enhancement", "help wanted", "Stellar Wave", "frontend", "complexity: trivial"],
            body,
        )
    )

LIVE = [
    ("LIVE-01", "Staging health probe includes indexer freshness header", "backend", "medium"),
    ("LIVE-02", "Quote endpoint exposes `data_quality` enum in OpenAPI only (doc + types)", "backend", "trivial"),
    ("LIVE-03", "Redis cache miss metric for quote path", "backend", "medium"),
    ("LIVE-04", "Frontend swap shows graceful stale quote banner copy", "frontend", "trivial"),
    ("LIVE-05", "sdk-js: export `DEFAULT_PUBLIC_HORIZON_URL` in package docs", "sdk-js", "trivial"),
    ("LIVE-06", "CLI `stellarroute health --json` exits non-zero on degraded", "cli", "medium"),
    ("LIVE-07", "Indexer logs SDEX cursor on restart", "indexer", "trivial"),
    ("LIVE-08", "Rate limit 429 body includes `retry_after_ms`", "backend", "medium"),
    ("LIVE-09", "OpenAPI documents swap `network_passphrase` on prepare", "docs", "trivial"),
    ("LIVE-10", "Playwright: swap happy path mock records tx hash", "frontend", "medium"),
    ("LIVE-11", "Rust SDK health method matches OpenAPI", "sdk-rust", "medium"),
    ("LIVE-12", "Contracts: document `PoolNotSupported` rename plan", "docs", "trivial"),
    ("LIVE-13", "Frontend wallet error mapper for Freighter reject", "frontend", "medium"),
    ("LIVE-14", "CCTP quote TTL shown in cross-chain UI", "frontend", "trivial"),
    ("LIVE-15", "Swap history page paginates beyond 50 rows client-side", "frontend", "medium"),
]

for code, short, area, complexity in LIVE:
    title = f"[live-path] {code} {short}"
    labels = ["enhancement", "help wanted", "Stellar Wave", "live-path", f"complexity: {complexity}"]
    if area == "frontend":
        labels.append("frontend")
    if area == "backend":
        labels.append("backend")
    body = fix_body(
        short,
        f"Production path gap: {short}. Grounded in AGENTS.md live-path goals.",
        f"`{area}/**` as scoped in PR description",
        f"Implement {short} with tests; keep frozen surfaces unchanged.",
        f"- Tests added or updated\n- Lean CI green for touched crates/packages\n- PR lists files and proves additive/frozen compliance",
    )
    ISSUES.append(("live", title, labels, body))

WAVE_SDK = [
    (f"W12-SDK-{i:02d}", t, c, a)
    for i, (t, c, a) in enumerate(
        [
            ("Rust SDK batch quote method", "medium", "crates/sdk-rust"),
            ("JS SDK StellarSdkNetwork exported in README", "trivial", "sdk-js"),
            ("JS SDK abortable confirmSwap documented", "trivial", "sdk-js"),
            ("Rust SDK CCTP types mirror OpenAPI", "high", "crates/sdk-rust"),
            ("sdk-js prepareSwap sends idempotency header option", "medium", "sdk-js"),
            ("CLI quote table output color off flag", "trivial", "crates/cli"),
            ("CLI orderbook JSON pretty print", "trivial", "crates/cli"),
            ("sdk-rust integration test for pairs endpoint", "medium", "crates/sdk-rust"),
            ("sdk-js example for batch orderbook", "trivial", "sdk-js/examples"),
            ("sdk-js type guard tests for card preview", "medium", "sdk-js"),
        ],
        start=1,
    )
]

for code, short, complexity, allowed in WAVE_SDK:
    title = f"[sdk] {code} {short}"
    labels = ["enhancement", "help wanted", "Stellar Wave", f"complexity: {complexity}"]
    body = fix_body(
        short,
        short,
        f"{allowed}/**",
        f"Deliver {short} with unit/integration tests.",
        "- Tests pass\n- README or rustdoc updated if public API changes",
    )
    ISSUES.append(("sdk", title, labels, body))

WAVE_FE = [
    (f"W12-FE-{i:02d}", t, c)
    for i, (t, c) in enumerate(
        [
            ("Swap slippage preset chips remember last choice", "trivial"),
            ("Orderbook depth chart aria labels", "medium"),
            ("Offramp institution search debounce", "trivial"),
            ("Cross-chain corridor selector keyboard nav", "medium"),
            ("Settings theme preview before apply", "trivial"),
            ("History CSV export button behind flag", "medium"),
            ("Wallet modal focus trap on open", "medium"),
            ("Analytics page skeleton loading", "trivial"),
            ("Pair selector recent pairs LRU", "medium"),
            ("Price impact tooltip accessible description", "trivial"),
            ("Guide page table of contents skip link", "trivial"),
            ("Header network badge testnet/mainnet", "trivial"),
            ("Swap amount input scientific notation guard", "medium"),
            ("Footer links open in new tab security attrs", "trivial"),
            ("404 page link back to /swap", "trivial"),
            ("Loading spinner reduced-motion fallback", "trivial"),
        ],
        start=1,
    )
]

for code, short, complexity in WAVE_FE:
    title = f"[frontend] {code} {short}"
    labels = ["enhancement", "help wanted", "Stellar Wave", "frontend", f"complexity: {complexity}"]
    body = fix_body(
        short,
        f"UX polish: {short}.",
        "frontend/**",
        f"Implement {short} with Vitest coverage.",
        "- Vitest tests for new behavior\n- No change to live swap CTA when flags off",
    )
    ISSUES.append(("fe", title, labels, body))

assert len(ISSUES) == 50, f"expected 50 issues, got {len(ISSUES)}"


def main() -> int:
    dry = "--dry-run" in sys.argv
    created: list[dict] = []
    for _prefix, title, labels, body in ISSUES:
        if dry:
            print(title)
            continue
        cmd = [
            "gh",
            "issue",
            "create",
            "-R",
            REPO,
            "--title",
            title,
            "--body",
            body,
        ]
        for lab in labels:
            cmd.extend(["--label", lab])
        out = subprocess.check_output(cmd, text=True).strip()
        num = out.rsplit("/", 1)[-1]
        created.append({"number": int(num), "title": title, "url": out})
        print(f"created #{num}: {title}")
        time.sleep(0.4)
    if not dry:
        path = "scripts/wave/wave12-created.json"
        with open(path, "w") as f:
            json.dump(created, f, indent=2)
        print(f"Wrote {path} ({len(created)} issues)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
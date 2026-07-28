# User Guide: Your First Live Swap on StellarRoute

**Issue:** #1021  
**Audience:** Non-developer traders  
**App path:** [/guide](https://stellarroute.vercel.app/guide) (also available in the swap help drawer via `?`)  
**Related:** [Empty states spec](./design/empty-states-spec.md), [Swap E2E flow](./swap-e2e-flow.md), [Risk disclosure](./risk-disclosure.md)

This short guide walks you through your **first live swap**: connect a wallet, set trustlines, pick a pair, set slippage, and confirm.

> **Network tip:** Confirm the footer network badge shows **Testnet** or **Mainnet** as intended before you swap. Testnet funds have no real value; mainnet swaps use real assets.

---

## Annotated steps (quick map)

```text
┌─────────────────────────────────────────────────────────────┐
│  StellarRoute Swap                                          │
│  ┌──────────────┐                                           │
│  │ [Connect] ①  │  ← Connect Freighter / xBull              │
│  └──────────────┘                                           │
│  ┌────────────────────────────┐                             │
│  │ Pay:    [ XLM ▼ ]  10  ②   │  ← Choose asset + amount    │
│  │ Receive:[ USDC▼ ]  …   ③   │  ← Quote fills automatically│
│  └────────────────────────────┘                             │
│  Slippage: 0.5%  ④   Route preview  ⑤                       │
│  [ Review & Swap ]  ⑥                                       │
└─────────────────────────────────────────────────────────────┘
```

| Step | What you do | Why it matters |
|------|-------------|----------------|
| ① | Connect wallet | Signs the Stellar transaction; StellarRoute never holds your keys |
| ② | Pick pay asset + amount | Starts the quote request across SDEX + AMM venues |
| ③ | Confirm receive asset | May require a **trustline** for non-XLM assets |
| ④ | Set slippage | Caps how much worse the fill can be vs the quote |
| ⑤ | Review route | Multi-hop paths can improve price but add complexity |
| ⑥ | Confirm in wallet | Final authorization happens in your wallet extension |

*(Screenshot placeholders: capture Connect → Quote → Confirm once UI is frozen for launch, and drop images into `docs/assets/first-swap/` as `01-connect.png` … `06-confirm.png`.)*

---

## Step-by-step

### 1. Install and fund a wallet

1. Install [Freighter](https://www.freighter.app/) or [xBull](https://xbull.app/).
2. Create or import an account.
3. **Testnet:** fund via [Friendbot](https://friendbot.stellar.org/) (needs your public address).  
   **Mainnet:** deposit XLM (and any assets you will sell) from an exchange or another wallet.
4. Keep a small XLM reserve for fees and base reserves (Stellar accounts need a minimum balance).

### 2. Connect on StellarRoute

1. Open the [Swap](/swap) page (home redirects here).
2. Click **Connect wallet** (onboarding checklist step 1, or the wallet control in the header).
3. Approve the connection in your wallet.
4. If you see a **network mismatch** banner, switch the wallet network to match the app footer.

### 3. Add a trustline (non-XLM receive assets)

Stellar requires a **trustline** before your account can hold most non-XLM assets (for example USDC).

1. Choose a receive asset that is not native XLM.
2. If the UI prompts you to **Create trustline**, review the asset code + issuer and confirm in your wallet.
3. Wait for confirmation, then return to the swap form.

Without a trustline, the receive leg can fail even when the quote looks fine.

### 4. Select a pair and enter an amount

1. Use the **Pay** selector to choose what you sell.
2. Use the **Receive** selector for what you want.
3. Enter a **small amount** for your first live swap.
4. Wait for the quote (“Finding best route…”). Review **price impact** and estimated receive amount.

If you see **No liquidity for this pair**, try another pair or a smaller size (see [empty states](./design/empty-states-spec.md)).

### 5. Set slippage tolerance

1. Open slippage settings on the swap card (or Settings).
2. Start with a modest default (for example **0.5%**). Use higher only if quotes fail due to movement.
3. Read any **high slippage** warnings carefully — higher tolerance increases the chance of a worse fill.

### 6. Review route and confirm

1. Expand the route preview if shown (hops, venues, fees).
2. Click **Review & Swap** / **Confirm**.
3. In your wallet, verify amounts, destination, and fees, then **sign**.
4. Track status in the UI; use the explorer link when a transaction hash appears.

---

## Checklist before your first live swap

- [ ] Wallet connected and on the correct network  
- [ ] Enough XLM for fees + reserves  
- [ ] Trustline set for the receive asset (if not XLM)  
- [ ] Slippage reviewed  
- [ ] Amount is intentionally small for a first try  
- [ ] You understand [aggregator risks](./risk-disclosure.md) (slippage, routing, smart contracts)

---

## Help from the product UI

| Surface | How to open the guide |
|---------|------------------------|
| Swap onboarding checklist | “First swap guide” link under the checklist |
| Keyboard help drawer | Press `?` on the swap card → “First live swap guide” |
| In-app page | `/guide` |
| Docs hub | Linked from `/docs` and this repository `docs/` index |

---

## Troubleshooting

| Symptom | What to try |
|---------|-------------|
| Wallet won’t connect | Refresh; unlock extension; check supported wallet list |
| Network mismatch | Align wallet network with app footer (Testnet vs Mainnet) |
| Trustline errors | Confirm asset issuer; ensure you can pay the trustline reserve |
| Quote errors / timeouts | Retry; check [/status](/status); try a smaller amount |
| Transaction fails after sign | Read wallet / Horizon error; check balance, trustlines, and slippage |

---

## Verify

```bash
ls docs | head
# expect user-guide-first-live-swap.md among docs entries
```

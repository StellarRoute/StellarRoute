# AI Agent preview enablement (preview only)

Operator note for enabling the agent on a preview without a production flip in the same change.

## Flags (both default false)

- Backend: `AI_AGENT_ENABLED` — default `false` (unset = disabled, API returns `404`).
- Frontend: `NEXT_PUBLIC_AI_AGENT` — default `false` (unset = disabled, `/ai` shows disabled state).
  - Issue text refers to `NEXT_PUBLIC_FLAG_AI_AGENT`; there is no `NEXT_PUBLIC_FLAG_AI_AGENT` in code or `.env.example`. Do not invent it — use `NEXT_PUBLIC_AI_AGENT` for a local preview and leave any `NEXT_PUBLIC_FLAG_AI_AGENT` unset.

## Enable for a preview (local / preview deployment only)

Backend (preview shell only, never commit):

```bash
AI_AGENT_ENABLED=true cargo run -p stellarroute-api
```

Frontend (preview shell only, never commit):

```bash
NEXT_PUBLIC_AI_AGENT=true npm --prefix frontend run dev
```

For a hosted preview (e.g. Vercel preview deployment): set `AI_AGENT_ENABLED=true` and `NEXT_PUBLIC_AI_AGENT=true` as preview-only environment variables on that preview deployment only.

## Production stays off

Production Vercel stays unset: do not set `AI_AGENT_ENABLED` or `NEXT_PUBLIC_AI_AGENT` (or `NEXT_PUBLIC_FLAG_AI_AGENT`) on production, in Vercel production env, in `.env.example`, or in any committed config. Do not edit the staging deploy workflow.

## Unchanged defaults

CCTP and swap defaults stay as they are: `CCTP_ENABLED` remains `false` by default, classic one-hop SDEX prepare → wallet sign → submit and quote ranking are untouched, and no workflow or `.env.example` value is set `true` by this note.

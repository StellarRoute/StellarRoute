# StellarRoute Documentation

This directory is the documentation hub for StellarRoute. It organizes design, deployment, API, SDK, contract, and development content so contributors can find the right guide quickly.

## Documentation categories

### Architecture

- [Architecture overview](architecture/README.md) — entry point for architecture and operational design.
- [Diagrams](architecture/diagrams.md) — system, data flow, and deployment diagrams.
- [Database schema](architecture/database-schema.md) — normalized liquidity model and ERD.
- [Performance notes](architecture/PERFORMANCE_NOTES.md) — performance guidance and optimization notes.
- [Worker pool](architecture/WORKER_POOL.md) — indexer worker architecture and ingestion design.
- [Reconciliation](architecture/RECONCILIATION.md) — data consistency and recovery strategy.
- [Multi-region architecture](architecture/MULTI_REGION_ARCHITECTURE.md) — geo-distributed system design.
- [Multi-region runbook](architecture/MULTI_REGION_RUNBOOK.md) — operational runbook for multi-region deployments.

### API

- [API overview](api/README.md) — entry point for API reference.
- [Routes endpoint](api/routes_endpoint.md) — quote, orderbook, and route REST endpoints.
- [WebSocket API](api/websocket.md) — real-time quote stream API.
- [Versioning](api/versioning.md) — API versioning strategy.
- [Versioning policy](api/versioning-policy.md) — lifecycle policy for API changes.
- [Error taxonomy](api/error_taxonomy.md) — standardized API error responses.
- [Canonical pair ordering](api/canonical_pair_ordering.md) — rules for normalizing asset pair order.
- [v1 migration guide](api/v1-migration-guide.md) — compatibility and migration advice.
- [OpenAPI spec](api/openapi.yaml) — machine-readable REST API schema.
- [Integrator guide](api/integrator-guide.md) — quick-start and integration patterns for external API consumers.
- [Integrator error guide](api/integrator-error-guide.md) — error handling and retry guidance for integrators.

### Contracts

- [Contracts overview](contracts/README.md) — entry point for Soroban contract docs.
- [Router interface](contracts/router-interface.md) — public contract interface and event schema.
- [Contract deployment runbook](contracts/deployment-runbook.md) — Soroban contract lifecycle.
- [Gas benchmarks](contracts/gas-benchmarks.md) — gas cost and performance benchmarks.
- [Gas optimization](contracts/gas-optimization-usage.md) — WASM and gas optimization guidance.

### Deployment

- [Deployment overview](deployment/README.md) — deployment and production guides.
- [DB pool tuning](deployment/db-pool-tuning.md) — PostgreSQL pool sizing and tuning.
- [Database timeout guardrails](deployment/database-timeout-guardrails.md) — runtime timeout strategies.
- [Tracing troubleshooting](deployment/tracing-troubleshooting.md) — observability and tracing guidance.

### Development

- [Setup guide](development/SETUP.md) — local environment setup and tooling.
- [Indexer guide](development/indexer-guide.md) — indexer runbook, configuration, and troubleshooting.
- [Testing guide](development/testing-guide.md) — project test strategies and commands.
- [Wallet integration](development/wallet-integration.md) — frontend wallet integration patterns.
- [Frontend developer guide](development/frontend-guide.md) — frontend setup, workflow, and contribution patterns.

### SDK

- [TypeScript SDK documentation](sdk-js/README.md) — TypeScript SDK guides and API docs.
- [Rust SDK documentation](sdk-rust/README.md) — Rust SDK usage and examples.

### Operational runbooks

- [Kill switch runbook](RUNBOOK_KILL_SWITCH.md) — emergency kill-switch activation and recovery procedure.
- [Quote purger runbook](QUOTE_PURGER_RUNBOOK.md) — quote purger operation, scheduling, and troubleshooting.
- [Routing canary](routing_canary.md) — canary routing configuration, promotion, and rollback.
- [Indexer lag monitoring](indexer-lag-monitoring.md) — indexer lag alerting thresholds and remediation steps.
- [Cache hierarchical invalidation](cache/hierarchical_invalidation.md) — cache invalidation strategy and failure modes.
- [Swap end-to-end flow](swap-e2e-flow.md) — walkthrough of a complete swap from quote to on-chain settlement.
- [Incident replay workflow](incident-replay-workflow.md) — replay and recovery workflow.
- [Audit log retention](audit-log-retention.md) — audit log retention policy.

### Supporting docs

- [Monitoring](monitoring.md) — monitoring and metrics guidance.
- [Consistency strategy](CONSISTENCY_STRATEGY.md) — data consistency guarantees and trade-offs.
- [Hybrid optimizer](hybrid_optimizer.md) — optimizer architecture and behavior.
- [Key rotation](key_rotation.md) — credential and key rotation procedures.
- [Performance budget](performance_budget.md) — frontend and API performance targets.
- [Readiness](readiness/M2_GUIDE.md) — readiness and runbook content.

### Design

- [Information architecture](design/information-architecture.md) — sitemap, navigation hierarchy, and UX structure.
- [Empty states spec](design/empty-states-spec.md) — design specification for empty and error states.
- [Accessibility contrast audit](design/accessibility-contrast-audit.md) — WCAG colour contrast audit results.

### Frontend documentation

These docs live under [`frontend/docs/`](../frontend/docs/) and cover frontend-specific contributor topics.

- [Motion design guidelines](../frontend/docs/motion-design-guidelines.md) — animation principles and approved motion patterns.
- [Telemetry schema](../frontend/docs/telemetry-schema.md) — frontend event tracking schema and naming conventions.
- [Trader error copy style guide](../frontend/docs/trader-error-copy-style-guide.md) — tone, phrasing, and copy standards for error messages.
- [Debug overlay](../frontend/docs/debug-overlay.md) — developer debug panel usage and extension guide.
- [Iconography system](../frontend/docs/iconography-system.md) — icon library conventions and usage rules.
- [Wallet onboarding wireframes](../frontend/docs/WALLET_ONBOARDING_WIREFRAMES.md) — wireframes for the wallet connection onboarding flow.
- [Quote refresh screen reader testing](../frontend/docs/QUOTE_REFRESH_SCREEN_READER_TESTING.md) — accessibility testing guide for live-region quote announcements.
- [Swap i18n audit](../frontend/docs/swap-i18n-audit.md) — internationalisation coverage audit for swap UI strings.
- [Swap visual regression](../frontend/docs/swap-visual-regression.md) — visual regression test setup and baseline management.
- [Price history sparkline](../frontend/docs/price-history.md) — implementation notes for the 24-hour price sparkline.
- [Orderbook highlighting](../frontend/docs/orderbook-highlighting-feature.md) — feature spec for orderbook row highlighting.
- [Hero CTA feature](../frontend/docs/hero-cta-feature.md) — implementation spec for the hero call-to-action component.
- [Relative time feature](../frontend/docs/relative-time-feature.md) — design and implementation of the relative timestamp component.
- [Status page feature](../frontend/docs/status-page-feature.md) — feature spec for the system status page.
- [Feature flags](../frontend/src/FEATURE_FLAGS.md) — runtime feature flag reference and usage guide.
- [Storybook guide](../frontend/STORYBOOK.md) — Storybook setup, story conventions, and CI snapshot workflow.

## Getting started

See the main [project README](../README.md) for an overview of StellarRoute.

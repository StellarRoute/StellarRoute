# StellarRoute Documentation

This directory is the documentation hub for StellarRoute. It organizes design, deployment, API, SDK, contract, development, and operations content so contributors and operators can find the right guide quickly.

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

### API and integrators

- [API overview](api/README.md) — entry point for API reference.
- [Routes endpoint](api/routes_endpoint.md) — quote, orderbook, and route REST endpoints.
- [WebSocket API](api/websocket.md) — real-time quote stream API.
- [Versioning](api/versioning.md) — API versioning strategy.
- [Versioning policy](api/versioning-policy.md) — lifecycle policy for API changes.
- [Error taxonomy](api/error_taxonomy.md) — standardized API error responses.
- [v1 migration guide](api/v1-migration-guide.md) — compatibility and migration advice.
- [Canonical pair ordering](api/canonical_pair_ordering.md) — asset pair normalization rules.
- [Integrator guide](api/integrator-guide.md) — integration checklist and request patterns.
- [Integrator error guide](api/integrator-error-guide.md) — client-facing errors, recovery flows, and retry guidance.
- [OpenAPI spec](api/openapi.yaml) — machine-readable REST API schema.

### Contracts

- [Contracts overview](contracts/README.md) — entry point for Soroban contract docs.
- [Router interface](contracts/router-interface.md) — public contract interface and event schema.
- [Contract deployment runbook](contracts/deployment-runbook.md) — Soroban contract lifecycle.
- [Gas benchmarks](contracts/gas-benchmarks.md) — gas cost and performance benchmarks.
- [Gas optimization](contracts/gas-optimization-usage.md) — WASM and gas optimization guidance.
- [Storage rent audit](contracts/storage-rent-audit.md) — Soroban storage footprint and rent review.
- [Contract testing guide](contracts/testing-guide.md) — contract-focused test strategy and commands.

### Deployment

- [Deployment overview](deployment/README.md) — deployment and production guides.
- [DB pool tuning](deployment/db-pool-tuning.md) — PostgreSQL pool sizing and tuning.
- [Database timeout guardrails](deployment/database-timeout-guardrails.md) — runtime timeout strategies.
- [Tracing troubleshooting](deployment/tracing-troubleshooting.md) — observability and tracing guidance.

### Development

- [Setup guide](development/SETUP.md) — local environment setup and tooling.
- [Testing guide](development/testing-guide.md) — project test strategies and commands.
- [Frontend onboarding](development/frontend-guide.md) — frontend setup, workflows, and conventions.
- [Indexer guide](development/indexer-guide.md) — indexer runbook and troubleshooting guidance.
- [Wallet integration](development/wallet-integration.md) — frontend wallet integration patterns.

### Operations and backend runbooks

- [Monitoring](monitoring.md) — monitoring and metrics guidance.
- [Routing canary](routing_canary.md) — routing canary rollout and validation workflow.
- [Hierarchical cache invalidation](cache/hierarchical_invalidation.md) — cache invalidation model and operator notes.
- [Indexer lag monitoring](indexer-lag-monitoring.md) — lag metrics, thresholds, and response playbook.
- [Kill switch runbook](RUNBOOK_KILL_SWITCH.md) — emergency control procedure for route execution.
- [Quote purger runbook](QUOTE_PURGER_RUNBOOK.md) — quote cleanup operations and safeguards.
- [Quote purger implementation](QUOTE_PURGER_IMPLEMENTATION.md) — purger design and implementation notes.
- [Swap end-to-end flow](swap-e2e-flow.md) — swap flow, system boundaries, and validation notes.
- [Incident replay workflow](incident-replay-workflow.md) — replay and recovery workflow.
- [Readiness guide](readiness/M2_GUIDE.md) — readiness and runbook content.
- [Audit log retention](audit-log-retention.md) — audit log retention policy.
- [Key rotation](key_rotation.md) — credential rotation procedure.
- [Performance budget](performance_budget.md) — frontend and API performance targets.
- [Consistency strategy](CONSISTENCY_STRATEGY.md) — cross-service consistency guarantees and tradeoffs.
- [Hybrid optimizer](hybrid_optimizer.md) — optimizer architecture and behavior.

### Design

- [Accessibility contrast audit](design/accessibility-contrast-audit.md) — color and contrast review notes.
- [Empty states spec](design/empty-states-spec.md) — empty, loading, and error-state UX guidance.
- [Information architecture](design/information-architecture.md) — navigation and content organization notes.

### Frontend documentation

Use the frontend docs for UI-specific implementation guidance, copy standards, and contributor workflows. These links intentionally point into `frontend/docs/` or `frontend/src/` so the source docs remain owned by the frontend package.

- [Feature flags](../frontend/src/FEATURE_FLAGS.md) — frontend feature flag catalog and rollout notes.
- [Telemetry schema](../frontend/docs/telemetry-schema.md) — analytics event names and payload contracts.
- [Trader error copy style guide](../frontend/docs/trader-error-copy-style-guide.md) — user-facing swap error copy standards.
- [Quote refresh screen reader testing](../frontend/docs/QUOTE_REFRESH_SCREEN_READER_TESTING.md) — assistive technology validation notes.
- [Wallet onboarding wireframes](../frontend/docs/WALLET_ONBOARDING_WIREFRAMES.md) — wallet onboarding flow references.
- [Debug overlay](../frontend/docs/debug-overlay.md) — quote/debug overlay behavior and metadata.
- [Hero CTA feature](../frontend/docs/hero-cta-feature.md) — landing hero CTA behavior.
- [Iconography system](../frontend/docs/iconography-system.md) — icon usage and token guidance.
- [Motion design guidelines](../frontend/docs/motion-design-guidelines.md) — animation timing and reduced-motion guidance.
- [Orderbook highlighting feature](../frontend/docs/orderbook-highlighting-feature.md) — orderbook row highlighting behavior.
- [Price history](../frontend/docs/price-history.md) — price history UI and data behavior.
- [Relative time feature](../frontend/docs/relative-time-feature.md) — relative timestamp formatting guidance.
- [Status page feature](../frontend/docs/status-page-feature.md) — status page behavior and states.
- [Swap i18n audit](../frontend/docs/swap-i18n-audit.md) — internationalization review notes for swap UI.
- [Swap visual regression](../frontend/docs/swap-visual-regression.md) — visual regression coverage for swap flows.

### SDK

- [TypeScript SDK documentation](sdk-js/README.md) — TypeScript SDK guides and API docs.
- [TypeScript SDK API reference](sdk-js/api/api-reference.md) — generated client API reference.
- [Rust SDK documentation](sdk-rust/README.md) — Rust SDK usage and examples.

## Getting started

See the main [project README](../README.md) for an overview of StellarRoute. Contributors should also review the root [CONTRIBUTING guide](../CONTRIBUTING.md) before opening a pull request.

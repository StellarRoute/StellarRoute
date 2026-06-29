# StellarRoute Development Roadmap

**Project Goal:** Build a unified DEX aggregator and best-price routing solution across Stellar DEX (SDEX) orderbook and Soroban AMM pools.

**Last Updated:** Initial creation  
**Status:** Planning Phase

---

## Overview

This roadmap outlines the complete development journey for StellarRoute, from initial prototype to production deployment. The project is organized into 5 major milestones (M1-M5), each building upon the previous phase to deliver a comprehensive DEX aggregation platform.

---

## Milestone Breakdown

### **M1: Prototype Indexer & API Endpoints (SDEX Only)** 
**Status:** ✅ Complete (100%)  
**Duration:** ~6-8 weeks  
**Priority:** Critical Foundation

#### Objectives
- Establish core indexing infrastructure for SDEX orderbooks
- Build foundational database schema for market data
- Create initial REST API endpoints
- Implement real-time orderbook synchronization
- Achieve <500ms API response latency

#### Technical Tasks

**Phase 1.1: Environment & Project Setup** ✅ **COMPLETE**
- [x] Set up Rust development environment
- [x] Install Soroban CLI and Rust toolchain (`rustup target add wasm32-unknown-unknown`)
- [x] Initialize project structure with workspace layout
- [x] Configure CI/CD pipelines (GitHub Actions)
- [x] Set up local development environment (Docker Compose for Postgres)
- [x] Create project documentation structure

**Phase 1.2: SDEX Indexer Development** ✅ **COMPLETE**
- [x] Research Stellar Horizon API endpoints for orderbook data
- [x] Design database schema for orderbook storage (Postgres)
  - Offers table (price, amount, timestamp, asset pairs)
  - Orderbook state table (snapshots, versioning)
  - Asset metadata table
- [x] Implement Horizon API client (using reqwest directly - no official Rust SDK)
- [x] Build orderbook indexer service
  - Real-time streaming from Horizon API (polling-based, SSE-ready)
  - Polling mode for historical data
  - Dual-mode support (polling & streaming)
- [x] Add error handling and retry logic (exponential backoff, 3 retries)
- [x] Implement data validation and sanitization (comprehensive validation)

**Phase 1.3: Database Layer** ✅ **COMPLETE**
- [x] Set up Postgres database migrations (sqlx migrations complete)
- [x] Create normalized schema for price feeds (assets + offers tables)
- [x] Implement connection pooling (sqlx PgPool configured)
- [x] Add database indexes for query performance (11 indexes added)
- [x] Create data archival strategy for historical data (archival table + functions)
- [x] Optimize query performance (materialized views, denormalized views)
- [x] Add database health monitoring (HealthMonitor, metrics tracking)

**Phase 1.4: REST API Foundation** ✅ **COMPLETE**
- [x] Choose API framework (Axum selected)
- [x] Implement core REST endpoints:
  - `GET /api/v1/pairs` - List available trading pairs
  - `GET /api/v1/orderbook/{base}/{quote}` - Get orderbook for pair
  - `GET /api/v1/quote/{base}/{quote}?amount={amount}` - Get price quote
  - `GET /health` - Health check endpoint
- [x] Add request validation and error handling (comprehensive ApiError types)
- [x] Implement rate limiting (RateLimitLayer middleware)
- [x] Create OpenAPI/Swagger documentation (utoipa + Swagger UI)
- [x] Add CORS support (tower-http)

**Phase 1.5: Performance & Testing** ✅ **COMPLETE**
- [x] Implement caching layer (Redis) for frequently accessed data
  - Redis cache manager with graceful fallback
  - Cache keys: pairs (10s TTL), orderbook (5s TTL), quotes (2s TTL)
  - Optional Redis support via environment variable
- [x] Add API response compression
  - Gzip compression via tower-http CompressionLayer
  - Automatic compression for responses > 1KB
- [x] Code quality improvements
  - Type annotations for Rust 2024 edition compatibility
  - Comprehensive unit tests (5 passing)
  - Cache key builder functions

#### Deliverables
- ✅ Working SDEX orderbook indexer
- ✅ REST API serving real-time orderbook data
- ✅ Database with normalized market data
- ✅ API documentation
- ✅ Test coverage ≥70%
- ✅ Performance benchmarks meeting <500ms latency

#### Success Criteria
- Indexer successfully syncs SDEX orderbooks in real-time
- API responds to quote requests in <500ms under load
- Zero data loss during network interruptions
- Comprehensive test coverage

---

### **M2: Soroban AMM Integration & Routing Engine**
**Status:** 🔴 Not Started  
**Duration:** ~8-10 weeks  
**Priority:** Critical Core Feature  
**Dependencies:** M1 completion

#### Objectives
- Integrate Soroban AMM pool queries into indexing system
- Build unified price aggregation layer combining SDEX + AMM data
- Implement pathfinding algorithm for optimal routing
- Calculate price impact and slippage
- Support multi-hop routing through multiple liquidity sources

#### Technical Tasks

**Phase 2.1: Soroban Integration**
- [ ] Set up Soroban development environment
- [ ] Research Soroban AMM contract interfaces
- [ ] Implement Soroban RPC client integration
- [ ] Build AMM pool state aggregator
  - Query pool reserves
  - Track pool creation/destruction events
  - Monitor pool parameter changes (fees, reserves)
- [ ] Design unified data model for SDEX + AMM liquidity
- [ ] Implement normalization layer for different liquidity sources

**Phase 2.2: Routing Engine Architecture**
- [ ] Design routing algorithm architecture
  - Graph representation of liquidity sources
  - Path search algorithms (Dijkstra/A* with modifications)
- [ ] Implement single-hop routing
  - Direct SDEX orderbook swap
  - Direct AMM pool swap
  - Price comparison logic
- [ ] Implement multi-hop routing
  - 2-hop paths (e.g., XLM → USDC → BTC)
  - N-hop path discovery (with max depth limits)
  - Intermediate asset selection heuristics
- [ ] Add path caching for common routes

**Phase 2.3: Price Impact & Slippage Calculation**
- [ ] Implement SDEX orderbook price impact calculation
  - Calculate based on orderbook depth
  - Model market depth impact
- [ ] Implement AMM price impact calculation
  - Constant product formula (x * y = k)
  - Handle variable fee structures
- [ ] Add slippage tolerance validation
- [ ] Implement price improvement scoring algorithm

**Phase 2.4: Unified Price Aggregation**
- [ ] Build price aggregation service
  - Combine SDEX and AMM quotes
  - Rank routes by best price
  - Handle stale data and time-based validation
- [ ] Implement quote expiration logic
- [ ] Add quote caching with TTL
- [ ] Create unified quote response format

**Phase 2.5: Testing & Optimization**
- [ ] Write unit tests for routing algorithms
- [ ] Write integration tests with mock AMM pools
- [ ] Performance testing of pathfinding (handle large graphs)
- [ ] Test edge cases:
  - Insufficient liquidity
  - Very large swap amounts
  - Extremely thin orderbooks
- [ ] Optimize routing algorithm for speed

#### Deliverables
- ✅ Soroban AMM pool integration
- ✅ Unified price aggregation API
- ✅ Multi-hop routing engine
- ✅ Price impact and slippage calculations
- ✅ Comprehensive test suite
- ✅ Performance benchmarks

#### Success Criteria
- Successfully query all active Soroban AMM pools
- Routing engine finds optimal paths in <100ms
- Price aggregation correctly combines SDEX and AMM data
- Multi-hop routing works for common asset pairs

---

### **M3: Smart Contracts & Soroban Deployment**
**Status:** 🔴 Not Started  
**Duration:** ~10-12 weeks  
**Priority:** High  
**Dependencies:** M2 completion

#### Objectives
- Develop Soroban smart contracts for router logic
- Implement AMM swap execution contracts
- Create contract interfaces for price quotes
- Deploy and test on Stellar Testnet
- Ensure contract security and audit readiness

#### Technical Tasks

**Phase 3.1: Contract Architecture Design**
- [ ] Design router contract interface
  - Function signatures for quote requests
  - Swap execution methods
  - Event definitions
- [ ] Design contract state structures
  - Route configuration
  - Fee structures
  - Authorization mechanisms
- [ ] Plan contract upgradeability strategy
- [ ] Design gas/compute unit optimization strategy

**Phase 3.2: Router Contract Development**
- [ ] Set up Soroban contract project structure
- [ ] Implement router contract core logic:
  - `get_quote(base, quote, amount)` - Query optimal route
  - `execute_swap(route, amount, min_output, recipient)` - Execute swap
  - `validate_route(route)` - Route validation
- [ ] Implement access control patterns
- [ ] Add comprehensive error handling
- [ ] Implement events for all critical operations
- [ ] Add price impact checks and slippage protection

**Phase 3.3: AMM Integration Contracts**
- [ ] Research existing Soroban AMM contract interfaces
- [ ] Build adapter contracts if needed
- [ ] Implement cross-contract invocation (CCI) patterns
- [ ] Handle contract call failures gracefully
- [ ] Implement atomic swap execution (all-or-nothing)

**Phase 3.4: Security & Best Practices**
- [ ] Follow Soroban Rust SDK security guidelines
- [ ] Implement input validation on all functions
- [ ] Add checked arithmetic everywhere (prevent overflow)
- [ ] Implement reentrancy protection if applicable
- [ ] Add access control and authorization checks
- [ ] Use bounded types where possible
- [ ] Run `cargo clippy -- -D warnings`
- [ ] Run `cargo audit` for dependency vulnerabilities

**Phase 3.5: Testing**
- [ ] Write comprehensive unit tests for contracts
  - Test all functions
  - Test error cases
  - Test edge cases
- [ ] Set up Soroban local testing harness
- [ ] Write integration tests:
  - End-to-end swap execution
  - Multi-hop swap flows
  - Error handling scenarios
- [ ] Property-based testing for routing logic
- [ ] Fuzzing for input validation

**Phase 3.6: Deployment & Scripts**
- [ ] Create deployment scripts using Soroban CLI
- [ ] Set up testnet deployment process
- [ ] Implement contract verification
- [ ] Create contract upgrade scripts
- [ ] Document deployment procedures
- [ ] Deploy to Stellar Testnet
- [ ] Run integration tests against testnet

**Phase 3.7: Audit Preparation**
- [ ] Complete security self-audit
- [ ] Document all contract functions
- [ ] Create audit checklist
- [ ] Prepare for external security audit

#### Deliverables
- ✅ Production-ready Soroban router contracts
- ✅ Contract test suite with ≥90% coverage
- ✅ Deployment scripts and documentation
- ✅ Contracts deployed to Stellar Testnet
- ✅ Security audit readiness

#### Success Criteria
- All contracts pass security review checklist
- Contracts execute swaps correctly on testnet
- Gas/compute costs are optimized
- Ready for external security audit

---

### **M4: Web UI & SDK Libraries**
**Status:** 🔴 Not Started  
**Duration:** ~10-12 weeks  
**Priority:** High  
**Dependencies:** M2, M3 completion

#### Objectives
- Build intuitive web application for end users
- Create JavaScript/TypeScript SDK for developers
- Develop Rust SDK for backend integrations
- Provide CLI utilities for power users
- Enable wallet integration for transactions

#### Technical Tasks

**Phase 4.1: Frontend Web UI**

*4.1.1: Project Setup*
- [ ] Choose frontend framework (React/Next.js recommended)
- [ ] Set up TypeScript configuration
- [ ] Configure build tooling (Vite/Webpack/Turbopack)
- [ ] Set up UI component library (shadcn/ui, Tailwind CSS)
- [ ] Configure routing and state management

*4.1.2: Core UI Components*
- [ ] Token pair selector component
- [ ] Amount input with validation
- [ ] Price quote display component
- [ ] Route visualization component (show path through markets)
- [ ] Price impact indicator
- [ ] Slippage tolerance selector
- [ ] Transaction confirmation modal

*4.1.3: Trading Interface*
- [ ] Swap interface layout
- [ ] Real-time price updates (WebSocket integration)
- [ ] Quote refresh mechanism
- [ ] Best route highlighting
- [ ] Alternative route display
- [ ] Trade simulation (preview output amount)

*4.1.4: Wallet Integration*
- [ ] Integrate Stellar wallet connectors (Freighter, XBull, etc.)
- [ ] Implement wallet connection flow
- [ ] Display connected wallet address
- [ ] Handle wallet disconnection
- [ ] Transaction signing and submission
- [ ] Transaction status tracking

*4.1.5: Advanced Features*
- [ ] Historical trade history display
- [ ] Market depth visualization
- [ ] #407 Price chart sparkline for selected pair (24h window) [Stellar Wave, Drips complexity: High]
- [ ] Settings page (slippage defaults, theme)
- [ ] Responsive mobile design
- [ ] Error handling and user feedback
- [ ] Loading states and animations

*4.1.6: Testing*
- [ ] Write unit tests for components
- [ ] Write integration tests for trading flows
- [ ] End-to-end testing (Playwright/Cypress)
- [ ] Accessibility testing (a11y)

**Phase 4.2: JavaScript/TypeScript SDK**

*4.2.1: SDK Structure*
- [ ] Initialize TypeScript SDK project
- [ ] Set up build pipeline (Rollup/ESBuild)
- [ ] Configure package.json for npm publishing
- [ ] Set up documentation generation (TypeDoc)

*4.2.2: Core SDK Methods*
- [ ] `StellarRouteClient` class initialization
- [ ] `getQuote(base, quote, amount)` - Get price quote
- [ ] `getOrderbook(base, quote)` - Get orderbook
- [ ] `executeSwap(params)` - Execute swap
- [ ] `getRoutes(base, quote, amount)` - Get all possible routes
- [ ] Event listeners for price updates (WebSocket)

*4.2.3: Utilities*
- [ ] Asset pair formatting utilities
- [ ] Amount formatting and parsing
- [ ] Price impact calculation helpers
- [ ] Route comparison utilities

*4.2.4: Documentation & Examples*
- [ ] API reference documentation
- [ ] Getting started guide
- [ ] Example code snippets
- [ ] Integration tutorials
- [ ] Publish to npm registry

**Phase 4.3: Rust SDK**

*4.3.1: SDK Structure*
- [ ] Initialize Rust library crate
- [ ] Design SDK API surface
- [ ] Set up cargo workspace if needed

*4.3.2: Core SDK Methods*
- [ ] Client struct with API endpoint configuration
- [ ] Async HTTP client implementation (reqwest)
- [ ] Type-safe API request/response types
- [ ] Quote fetching methods
- [ ] Orderbook query methods
- [ ] Swap execution helpers

*4.3.3: Documentation*
- [ ] Rustdoc documentation
- [ ] Usage examples
- [ ] Integration guide
- [ ] Publish to crates.io

**Phase 4.4: CLI Utilities**

*4.4.1: CLI Tool*
- [ ] Use `clap` for argument parsing
- [ ] Implement commands:
  - `stellarroute quote <base> <quote> <amount>`
  - `stellarroute orderbook <base> <quote>`
  - `stellarroute routes <base> <quote> <amount>`
  - `stellarroute swap <route> <amount> --keyfile <path>`
- [ ] Add output formatting (JSON, table, human-readable)
- [ ] Error handling and user-friendly messages

*4.4.2: Distribution*
- [ ] Create installation scripts
- [ ] Cross-compilation for multiple platforms
- [ ] Release binaries via GitHub Releases

#### Deliverables
- ✅ Production-ready web UI
- ✅ Published JavaScript/TypeScript SDK (npm)
- ✅ Published Rust SDK (crates.io)
- ✅ CLI tool with cross-platform binaries
- ✅ Comprehensive SDK documentation
- ✅ Example applications

#### Success Criteria
- Web UI is intuitive and responsive
- SDKs are easy to integrate
- CLI tool is functional for power users
- All documentation is clear and accurate

---

### **M5: Audits, Documentation & Ecosystem Demos**
**Status:** 🔴 Not Started  
**Duration:** ~8-10 weeks  
**Priority:** Critical for Launch  
**Dependencies:** M3, M4 completion

#### Objectives
- Complete security audits of smart contracts
- Finalize all documentation
- Create ecosystem demonstrations
- Prepare for mainnet launch
- Establish monitoring and alerting

#### Technical Tasks

**Phase 5.1: Security Audits**

*5.1.1: Internal Audit*
- [ ] Complete final security review checklist
- [ ] Review all smart contract code
- [ ] Review backend API security
- [ ] Review SDK security considerations
- [ ] Fix all identified issues

*5.1.2: External Audit*
- [ ] Select reputable audit firm
- [ ] Prepare audit materials:
  - Code documentation
  - Architecture diagrams
  - Security assumptions
  - Test coverage reports
- [ ] Coordinate audit process
- [ ] Address audit findings
- [ ] Re-audit if necessary

*5.1.3: Bug Bounty Program (Optional)*
- [ ] Set up bug bounty program
- [ ] Define scope and rewards
- [ ] Deploy to testnet for testing
- [ ] Monitor and respond to submissions

**Phase 5.2: Documentation**

*5.2.1: User Documentation*
- [ ] Complete README with setup instructions
- [ ] Architecture overview documentation
- [ ] API reference documentation
- [ ] Web UI user guide
- [ ] SDK integration guides
- [ ] Troubleshooting guide

*5.2.2: Developer Documentation*
- [ ] Contract interface documentation
- [ ] SDK API reference
- [ ] Contribution guidelines
- [ ] Development setup guide
- [ ] Testing guide
- [ ] Deployment procedures

*5.2.3: Security Documentation*
- [ ] Security considerations document
- [ ] Audit reports
- [ ] Known limitations
- [ ] Risk disclosures

**Phase 5.3: Testing & Quality Assurance**

*5.3.1: Final Testing*
- [ ] End-to-end system testing
- [ ] Load testing of complete system
- [ ] Security penetration testing
- [ ] User acceptance testing
- [ ] Mobile device testing

*5.3.2: Testnet Deployment*
- [ ] Deploy all components to testnet
- [ ] Run extended testnet operations (1-2 weeks)
- [ ] Monitor performance and stability
- [ ] Fix any discovered issues

**Phase 5.4: Monitoring & Operations**

*5.4.1: Monitoring Setup*
- [ ] Set up application monitoring (e.g., DataDog, New Relic)
- [ ] Configure alerting for critical metrics
- [ ] Set up log aggregation
- [ ] Create dashboards for key metrics:
  - API latency
  - Error rates
  - Indexer sync status
  - Contract execution success rates

*5.4.2: Operations Documentation*
- [ ] Runbook for common operations
- [ ] Incident response procedures
- [ ] Backup and recovery procedures
- [ ] Scaling procedures

**Phase 5.5: Ecosystem Demos**

*5.5.1: Demo Applications*
- [ ] Create example dApp using StellarRoute SDK
- [ ] Build demo showcasing multi-hop routing
- [ ] Create tutorial videos
- [ ] Write blog posts about StellarRoute

*5.5.2: Community Engagement*
- [ ] Publish project to Stellar ecosystem directories
- [ ] Engage with Stellar developer community
- [ ] Present at Stellar meetups/events (if applicable)
- [ ] Create developer onboarding materials

**Phase 5.6: Mainnet Launch Preparation**

*5.6.1: Pre-Launch Checklist*
- [ ] All audits completed and issues resolved
- [ ] Documentation complete
- [ ] Monitoring and alerting operational
- [ ] Team trained on operations
- [ ] Incident response plan ready
- [ ] Legal and compliance review (if needed)

*5.6.2: Gradual Rollout*
- [ ] Deploy to mainnet with limited features
- [ ] Monitor closely for first 24-48 hours
- [ ] Gradually enable full functionality
- [ ] Collect user feedback
- [ ] Iterate based on feedback

#### Deliverables
- ✅ Completed security audits
- ✅ Comprehensive documentation
- ✅ Production monitoring and alerting
- ✅ Demo applications and tutorials
- ✅ Mainnet deployment
- ✅ Post-launch support plan

#### Success Criteria
- All security audits passed
- Documentation is complete and accurate
- System is stable on mainnet
- Community is engaged and using the platform

---

## Technical Architecture Summary

### Core Components

1. **Indexer Service (Rust)**
   - SDEX orderbook synchronization
   - Soroban AMM pool state aggregation
   - Real-time data streaming
   - Database persistence

2. **Routing Engine (Rust)**
   - Pathfinding algorithms
   - Price aggregation logic
   - Multi-hop route discovery
   - Price impact calculations

3. **API Server (Rust)**
   - REST/GraphQL endpoints
   - WebSocket for real-time updates
   - Caching layer
   - Rate limiting

4. **Smart Contracts (Soroban/Rust)**
   - Router contract
   - AMM integration contracts
   - Swap execution logic

5. **Frontend Web UI (React/TypeScript)**
   - User interface
   - Wallet integration
   - Real-time price display
   - Transaction management

6. **SDKs**
   - JavaScript/TypeScript SDK
   - Rust SDK
   - CLI tool

### Technology Stack

- **Backend:** Rust (Actix-web/Axum, Postgres, Redis)
- **Smart Contracts:** Soroban Rust SDK
- **Frontend:** React/Next.js, TypeScript, Tailwind CSS
- **Database:** PostgreSQL
- **Caching:** Redis
- **Monitoring:** Prometheus, Grafana (or similar)
- **Testing:** Rust test framework, Jest, Playwright

---

## Risk Mitigation

| Risk | Mitigation Strategy |
|------|-------------------|
| Market liquidity variance | Implement fallback routing, price smoothing logic |
| Contract vulnerabilities | Multiple audit rounds, comprehensive testing, bug bounty |
| Indexer bottleneck | Scalable storage, caching layers, horizontal scaling |
| API performance issues | Load testing, optimization, caching strategies |
| Soroban integration complexity | Early prototyping, community engagement |
| Data synchronization issues | Robust error handling, retry logic, state management |

---

## Success Metrics

### Technical Metrics
- API latency <500ms (p95)
- Indexer sync lag <1 second
- Contract execution success rate >99%
- Test coverage ≥70% (contracts ≥90%)

### Business Metrics
- Number of active trading pairs supported
- Volume routed through platform
- Developer SDK adoption
- User retention and satisfaction

---

## Timeline Estimate

| Milestone | Duration | Cumulative |
|-----------|----------|------------|
| M1 | 6-8 weeks | 6-8 weeks |
| M2 | 8-10 weeks | 14-18 weeks |
| M3 | 10-12 weeks | 24-30 weeks |
| M4 | 10-12 weeks | 34-42 weeks |
| M5 | 8-10 weeks | 42-52 weeks |

**Total Estimated Duration:** 42-52 weeks (~10-12 months)

---

## Next Steps

1. **Immediate Actions:**
   - Review and approve this roadmap
   - Set up development environment
   - Create GitHub repository structure
   - Begin M1 Phase 1.1 tasks

2. **Planning Files to Create:**
   - `task_plan.md` - Detailed task breakdown (following planning-with-files approach)
   - `findings.md` - Research notes and discoveries
   - `progress.md` - Session logs and progress tracking

3. **First Milestone Focus:**
   - Complete environment setup
   - Research Stellar Horizon API
   - Design database schema
   - Begin SDEX indexer implementation

---

## Notes

- This roadmap is a living document and should be updated as the project progresses
- Dependencies between milestones are critical - do not skip ahead
- Security is a priority at every stage, not just M5
- Community feedback should be incorporated throughout development
- Regular progress reviews and milestone checkpoints are recommended

---

**Questions or Updates?** Please update this roadmap file and communicate changes to the team.

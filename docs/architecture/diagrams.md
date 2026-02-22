# StellarRoute Architecture Diagrams

This document contains visual diagrams explaining the architecture, data flow, and deployment of StellarRoute.

## Table of Contents

- [System Architecture](#system-architecture)
- [Data Flow Diagram](#data-flow-diagram)
- [Deployment Architecture](#deployment-architecture)
- [Database Schema (ERD)](#database-schema-erd)
- [Component Interaction Sequence](#component-interaction-sequence)

---

## System Architecture

This diagram shows the high-level components of StellarRoute and how they interact.

```mermaid
graph TB
    subgraph "External Systems"
        StellarNetwork[Stellar Network<br/>Horizon API]
        SorobanRPC[Soroban RPC]
        Wallets[User Wallets<br/>Freighter, XBull]
    end

    subgraph "StellarRoute Platform"
        subgraph "Backend Services"
            Indexer[Indexer Service<br/>Rust]
            Router[Routing Engine<br/>Rust]
            API[API Server<br/>Axum/Rust]
            Contracts[Smart Contracts<br/>Soroban/Rust]
        end

        subgraph "Data Layer"
            PostgreSQL[(PostgreSQL<br/>Orderbook Data)]
            Redis[(Redis<br/>Cache Layer)]
        end

        subgraph "Frontend"
            WebUI[Web UI<br/>React/Next.js]
            SDK_TS[TypeScript SDK]
            SDK_Rust[Rust SDK]
        end
    end

    subgraph "External Clients"
        Traders[Traders]
        Developers[Third-party<br/>Developers]
        dApps[dApps]
    end

    %% External to Backend
    StellarNetwork -->|SDEX Offers<br/>Orderbook Data| Indexer
    SorobanRPC -->|AMM Pool States| Indexer

    %% Backend Flow
    Indexer -->|Store Offers| PostgreSQL
    Indexer -->|Notify Updates| Router
    Router -->|Query Orderbook| PostgreSQL
    Router -->|Calculate Routes| API
    API -->|Cache Results| Redis
    API -->|Query Data| PostgreSQL

    %% Contract Integration
    Contracts -->|Execute Swaps| SorobanRPC
    API -->|Route Info| Contracts

    %% Frontend to Backend
    WebUI -->|REST/WebSocket| API
    SDK_TS -->|API Calls| API
    SDK_Rust -->|API Calls| API

    %% Users to Frontend
    Traders -->|Trade UI| WebUI
    Developers -->|Integrate| SDK_TS
    Developers -->|Integrate| SDK_Rust
    dApps -->|Use SDKs| SDK_TS

    %% Wallet Integration
    WebUI -->|Sign Transactions| Wallets
    Wallets -->|Submit| StellarNetwork

    style Indexer fill:#e1f5ff
    style Router fill:#e1f5ff
    style API fill:#e1f5ff
    style Contracts fill:#ffe1f5
    style PostgreSQL fill:#fff4e1
    style Redis fill:#ffe1e1
    style WebUI fill:#e1ffe1
```

### Components Description

**Indexer Service**

- Continuously syncs SDEX orderbook data from Horizon API
- Indexes Soroban AMM pool states
- Normalizes and stores data in PostgreSQL
- Implements retry logic and rate limiting

**Routing Engine**

- Implements pathfinding algorithms (Dijkstra, A\*)
- Calculates optimal multi-hop routes
- Considers price impact and slippage
- Evaluates SDEX + AMM combinations

**API Server**

- RESTful endpoints for quotes and orderbook data
- WebSocket support for real-time updates
- Rate limiting and caching
- OpenAPI/Swagger documentation

**Smart Contracts**

- Soroban-based router contracts
- Atomic swap execution
- Multi-hop routing on-chain
- Slippage protection

---

## Data Flow Diagram

This diagram illustrates how data flows through the system from ingestion to user interaction.

```mermaid
flowchart TD
    Start([User Requests Quote]) --> API_Request[API Receives Request<br/>GET /api/v1/quote/XLM/USDC?amount=100]

    API_Request --> Cache_Check{Check Redis Cache}
    Cache_Check -->|Cache Hit| Return_Cached[Return Cached Result]
    Cache_Check -->|Cache Miss| Query_DB

    Query_DB[Query PostgreSQL<br/>for Orderbook Data] --> Route_Calc[Routing Engine<br/>Calculate Best Path]

    Route_Calc --> Multi_Hop{Multi-Hop<br/>Required?}

    Multi_Hop -->|No| Direct_Route[Direct Route<br/>XLM → USDC]
    Multi_Hop -->|Yes| Complex_Route[Multi-Hop Route<br/>XLM → BTC → USDC]

    Direct_Route --> Price_Calc[Calculate Price Impact<br/>& Slippage]
    Complex_Route --> Price_Calc

    Price_Calc --> Store_Cache[Store Result in Redis<br/>TTL: 2 seconds]
    Store_Cache --> Return_Quote[Return Quote Response]
    Return_Cached --> Display
    Return_Quote --> Display[Display to User]

    Display --> User_Decision{User Accepts?}
    User_Decision -->|No| End([End])
    User_Decision -->|Yes| Build_TX[Build Transaction]

    Build_TX --> Sign_TX[User Signs with Wallet]
    Sign_TX --> Submit[Submit to Stellar Network]
    Submit --> Execute[Execute Swap<br/>via Smart Contract]
    Execute --> Confirm[Transaction Confirmed]
    Confirm --> Update_DB[Update Database<br/>Refresh Orderbook]
    Update_DB --> End

    subgraph "Background Process"
        Horizon[Horizon API] -->|Poll Every 2s| Indexer_Sync[Indexer Service]
        Indexer_Sync --> Validate[Validate & Normalize Data]
        Validate --> Upsert[Upsert into PostgreSQL]
        Upsert --> Notify[Notify WebSocket Clients]
    end

    style Cache_Check fill:#ffe1e1
    style Query_DB fill:#fff4e1
    style Route_Calc fill:#e1f5ff
    style Store_Cache fill:#ffe1e1
    style Execute fill:#ffe1f5
```

### Data Flow Stages

1. **Request Phase**: User requests quote through API
2. **Cache Layer**: Check Redis for recent results
3. **Query Phase**: Fetch orderbook from PostgreSQL
4. **Routing Phase**: Calculate optimal path considering multi-hop
5. **Response Phase**: Return quote with price impact
6. **Execution Phase**: User signs and submits transaction
7. **Background Sync**: Continuous indexing from Horizon

---

## Deployment Architecture

This diagram shows the deployment topology and infrastructure components.

```mermaid
graph TB
    subgraph "Internet"
        Users[Users/Traders]
        Devs[Third-party Apps]
    end

    subgraph "Load Balancer / CDN"
        LB[Nginx / Cloudflare<br/>SSL Termination]
    end

    subgraph "Application Tier - Docker Containers"
        subgraph "API Cluster"
            API1[API Server 1<br/>:3000]
            API2[API Server 2<br/>:3000]
            API3[API Server 3<br/>:3000]
        end

        subgraph "Indexer Cluster"
            Indexer1[Indexer 1<br/>Primary]
            Indexer2[Indexer 2<br/>Standby]
        end

        subgraph "Frontend"
            Frontend[Next.js App<br/>Static + SSR]
        end
    end

    subgraph "Data Tier"
        subgraph "Database Cluster"
            PG_Primary[(PostgreSQL<br/>Primary<br/>:5432)]
            PG_Replica[(PostgreSQL<br/>Read Replica<br/>:5432)]
        end

        subgraph "Cache Cluster"
            Redis_Master[(Redis Master<br/>:6379)]
            Redis_Replica[(Redis Replica<br/>:6379)]
        end
    end

    subgraph "Monitoring & Logging"
        Prometheus[Prometheus<br/>Metrics]
        Grafana[Grafana<br/>Dashboards]
        Loki[Loki<br/>Logs]
    end

    subgraph "External Services"
        Horizon[Stellar Horizon API<br/>horizon.stellar.org]
        Soroban[Soroban RPC<br/>soroban-rpc.stellar.org]
    end

    %% Traffic Flow
    Users --> LB
    Devs --> LB
    LB --> Frontend
    LB --> API1
    LB --> API2
    LB --> API3

    %% API to Data
    API1 --> Redis_Master
    API2 --> Redis_Master
    API3 --> Redis_Master
    API1 --> PG_Primary
    API2 --> PG_Replica
    API3 --> PG_Replica

    %% Indexer to Data
    Indexer1 --> PG_Primary
    Indexer2 --> PG_Primary
    Indexer1 --> Horizon
    Indexer2 --> Horizon
    Indexer1 --> Soroban

    %% Data Replication
    PG_Primary -.->|Streaming<br/>Replication| PG_Replica
    Redis_Master -.->|Replication| Redis_Replica

    %% Monitoring
    API1 --> Prometheus
    API2 --> Prometheus
    API3 --> Prometheus
    Indexer1 --> Prometheus
    PG_Primary --> Prometheus
    Redis_Master --> Prometheus
    Prometheus --> Grafana
    API1 --> Loki
    Indexer1 --> Loki

    style LB fill:#e1f5ff
    style API1 fill:#e1ffe1
    style API2 fill:#e1ffe1
    style API3 fill:#e1ffe1
    style Indexer1 fill:#e1ffe1
    style PG_Primary fill:#fff4e1
    style Redis_Master fill:#ffe1e1
    style Prometheus fill:#f5e1ff
    style Frontend fill:#e1f5ff
```

### Deployment Details

**Infrastructure Layer**

- Load balancer with SSL termination (Nginx/Cloudflare)
- Horizontal scaling for API servers (3+ instances)
- Container orchestration with Docker Compose/Kubernetes

**Application Layer**

- API servers: Stateless, can scale horizontally
- Indexer service: Primary/standby setup for high availability
- Frontend: Static assets + server-side rendering

**Data Layer**

- PostgreSQL: Primary-replica setup for read scaling
- Redis: Master-replica for cache availability
- Connection pooling for efficient database access

**Monitoring Stack**

- Prometheus: Metrics collection
- Grafana: Visualization dashboards
- Loki: Centralized logging

**High Availability**

- Multiple API instances behind load balancer
- Database replication for failover
- Cache replication for redundancy
- Health checks and auto-restart

---

## Database Schema (ERD)

Complete entity relationship diagram showing all tables and their relationships.

```mermaid
erDiagram
    assets ||--o{ sdex_offers : "selling_asset"
    assets ||--o{ sdex_offers : "buying_asset"
    assets ||--o{ trading_pairs : "base_asset"
    assets ||--o{ trading_pairs : "counter_asset"
    trading_pairs ||--o{ orderbook_snapshots : "has"

    assets {
        uuid id PK
        text asset_type
        text asset_code
        text asset_issuer
        timestamptz created_at
    }

    sdex_offers {
        bigint offer_id PK
        text seller
        uuid selling_asset_id FK
        uuid buying_asset_id FK
        numeric amount
        numeric price
        bigint price_n
        bigint price_d
        bigint last_modified_ledger
        text paging_token
        timestamptz updated_at
    }

    trading_pairs {
        uuid id PK
        uuid base_asset_id FK
        uuid counter_asset_id FK
        boolean is_active
        integer total_offers
        numeric total_volume
        timestamptz last_trade_at
        timestamptz created_at
        timestamptz updated_at
    }

    orderbook_snapshots {
        uuid id PK
        uuid trading_pair_id FK
        timestamptz snapshot_time
        jsonb bids
        jsonb asks
        integer bid_count
        integer ask_count
        numeric spread
        numeric mid_price
        numeric total_bid_volume
        numeric total_ask_volume
        bigint ledger_sequence
        timestamptz created_at
    }

    archived_offers {
        bigint offer_id PK
        text seller
        text selling_asset_type
        text selling_asset_code
        text selling_asset_issuer
        text buying_asset_type
        text buying_asset_code
        text buying_asset_issuer
        numeric amount
        numeric price
        timestamptz archived_at
        text archive_reason
    }

    ingestion_state {
        text key PK
        text value
        timestamptz updated_at
    }

    db_health_metrics {
        uuid id PK
        text metric_name
        numeric metric_value
        text metric_unit
        jsonb metadata
        timestamptz recorded_at
    }
```

### Key Relationships

- **assets → sdex_offers**: One asset can be in many offers (both as selling and buying)
- **assets → trading_pairs**: Each trading pair has a base and counter asset
- **trading_pairs → orderbook_snapshots**: Each pair can have multiple historical snapshots
- **archived_offers**: Standalone table for historical data (denormalized)

### Index Strategy

Primary indexes on all tables support:

- Fast lookups by primary key
- Efficient joins on foreign keys
- Time-series queries (timestamps)
- Price-based sorting for orderbook queries

---

## Component Interaction Sequence

Detailed sequence diagram showing how components interact during a typical quote request.

```mermaid
sequenceDiagram
    actor User
    participant WebUI
    participant API as API Server
    participant Cache as Redis
    participant DB as PostgreSQL
    participant Router as Routing Engine
    participant Indexer
    participant Horizon as Stellar Horizon

    Note over Indexer,Horizon: Background Process (Every 2s)
    Horizon->>Indexer: Stream offer updates
    Indexer->>Indexer: Validate & normalize
    Indexer->>DB: Upsert offers

    Note over User,Router: Quote Request Flow
    User->>WebUI: Enter trade details<br/>XLM → USDC, 100 XLM
    WebUI->>API: GET /api/v1/quote/XLM/USDC?amount=100

    API->>Cache: Check cached quote
    alt Cache Hit
        Cache-->>API: Return cached data
        API-->>WebUI: Quote response (2ms)
    else Cache Miss
        Cache-->>API: No cached data
        API->>DB: Query orderbook<br/>SELECT * FROM sdex_offers<br/>WHERE selling='XLM' AND buying='USDC'
        DB-->>API: Return offers [...]

        API->>Router: Calculate optimal route
        Router->>Router: Run pathfinding algorithm
        Router->>Router: Calculate price impact
        Router->>Router: Evaluate multi-hop options
        Router-->>API: Best route + price

        API->>Cache: Store result (TTL: 2s)
        API-->>WebUI: Quote response (150ms)
    end

    WebUI->>User: Display quote + route

    opt User Accepts Trade
        User->>WebUI: Confirm trade
        WebUI->>WebUI: Build transaction
        WebUI->>User: Request signature
        User->>WebUI: Sign with wallet
        WebUI->>Horizon: Submit transaction
        Horizon-->>WebUI: Transaction hash
        WebUI->>User: Show confirmation

        Note over Indexer: Detects new transaction
        Indexer->>DB: Update orderbook
        Indexer->>API: Notify via WebSocket
        API->>WebUI: Push update
        WebUI->>User: Refresh UI
    end
```

### Interaction Patterns

**Quote Request (Happy Path)**

1. User enters trade parameters in UI
2. API checks Redis cache (2s TTL)
3. On cache miss, queries PostgreSQL
4. Routing engine calculates best path
5. Result cached and returned to user
6. Total latency: ~150ms (cache miss), ~2ms (cache hit)

**Background Indexing**

1. Indexer polls Horizon API every 2 seconds
2. Validates and normalizes offer data
3. Upserts into PostgreSQL
4. Notifies WebSocket clients of updates

**Trade Execution**

1. User confirms trade parameters
2. Frontend builds transaction
3. User signs with wallet (Freighter/XBull)
4. Transaction submitted to Stellar network
5. Indexer detects change and updates database
6. Real-time notification to connected clients

---

## Diagram Source Files

All diagrams in this document are created using [Mermaid](https://mermaid.js.org/), a markdown-based diagramming tool that renders in GitHub, GitLab, and most modern documentation platforms.

### How to Edit

1. **In GitHub**: Diagrams render automatically in `.md` files
2. **Live Editor**: Use [Mermaid Live Editor](https://mermaid.live/) to modify diagrams
3. **VS Code**: Install "Markdown Preview Mermaid Support" extension
4. **Export**: Use Mermaid CLI or live editor to export as SVG/PNG

### Syntax Reference

````markdown
````mermaid
graph TD
    A[Start] --> B[Process]
    B --> C[End]
\```
````
````

### Updating Diagrams

When updating system architecture:

1. Edit the Mermaid code in this file
2. Verify rendering in GitHub preview
3. Update related documentation if component relationships change
4. Commit changes with descriptive message

---

## Additional Resources

- [Database Schema Details](./database-schema.md)
- [API Documentation](../api/README.md)
- [Deployment Guide](../deployment/README.md)
- [Development Setup](../development/SETUP.md)

---

**Last Updated**: February 20, 2026  
**Maintained By**: StellarRoute Team

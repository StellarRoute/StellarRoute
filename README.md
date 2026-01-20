# StellarRoute

**Open-source DEX aggregation engine and UI that delivers best-price routing across the Stellar DEX (SDEX) orderbook and Soroban AMM pools.**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Soroban](https://img.shields.io/badge/soroban-enabled-purple.svg)](https://soroban.stellar.org)

---

## 🚀 Overview

StellarRoute is a comprehensive DEX aggregation platform built for the Stellar ecosystem. It combines liquidity from both the traditional Stellar DEX (SDEX) orderbook and modern Soroban-based AMM pools to provide users with the best possible swap prices through intelligent multi-hop routing.

### What We're Building

- **Unified Liquidity Aggregation**: Index and aggregate liquidity from SDEX orderbooks and Soroban AMM pools
- **Intelligent Routing Engine**: Multi-hop pathfinding algorithm that discovers optimal trade routes across multiple liquidity sources
- **Smart Contracts**: Soroban-based router contracts for secure, on-chain swap execution
- **Developer SDKs**: Easy-to-use JavaScript/TypeScript and Rust SDKs for integrations
- **Web Interface**: Modern, intuitive UI for traders with real-time price updates and wallet integration
- **High Performance**: Sub-500ms API response times with real-time orderbook synchronization

---

## ✨ Key Features

### Core Capabilities
- ✅ **Best Price Discovery**: Automatically find the best execution price across all liquidity sources
- ✅ **Multi-Hop Routing**: Support for complex multi-step trades (e.g., XLM → USDC → BTC)
- ✅ **Price Impact Analysis**: Real-time calculation of price impact and slippage
- ✅ **Real-Time Indexing**: Continuous synchronization of SDEX and AMM pool states
- ✅ **Developer-Friendly**: Comprehensive SDKs and APIs for easy integration

### For Traders
- Execute swaps at the best available prices
- Visualize trade routes and price impact before execution
- Access deep liquidity across multiple sources
- Set custom slippage tolerance

### For Developers
- REST API for price quotes and orderbook data
- WebSocket support for real-time updates
- JavaScript/TypeScript SDK for web applications
- Rust SDK for backend integrations
- CLI tools for power users

---

## 🏗️ Architecture

StellarRoute is built with a modular architecture consisting of several key components:

### Backend Components (Rust)
```
┌─────────────────────────────────────────────────────┐
│                   StellarRoute                      │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────┐ │
│  │   Indexer    │  │   Routing    │  │   API    │ │
│  │   Service    │─▶│   Engine     │─▶│  Server  │ │
│  └──────────────┘  └──────────────┘  └──────────┘ │
│         │                                  │        │
│         ▼                                  ▼        │
│  ┌──────────────┐                   ┌──────────┐  │
│  │  PostgreSQL  │                   │  Redis   │  │
│  │   Database   │                   │  Cache   │  │
│  └──────────────┘                   └──────────┘  │
│                                                     │
└─────────────────────────────────────────────────────┘
         ▲                                    │
         │                                    │
    ┌────┴────┐                          ┌───▼────┐
    │ Stellar │                          │  Web   │
    │ Network │                          │   UI   │
    └─────────┘                          └────────┘
```

1. **Indexer Service**: Syncs SDEX orderbooks and Soroban AMM pool states from Stellar Horizon API
2. **Routing Engine**: Pathfinding algorithms for optimal multi-hop route discovery
3. **API Server**: REST/WebSocket endpoints serving quotes and orderbook data
4. **Smart Contracts**: Soroban contracts for on-chain swap execution
5. **Frontend UI**: React-based web interface for traders
6. **SDKs**: TypeScript and Rust libraries for developers

---

## 🛠️ Technology Stack

### Backend
- **Language**: Rust (for performance and safety)
- **Framework**: Axum/Actix-web (API server)
- **Database**: PostgreSQL (orderbook storage)
- **Cache**: Redis (hot data caching)
- **Blockchain**: Soroban (smart contracts)

### Frontend
- **Framework**: React/Next.js
- **Language**: TypeScript
- **Styling**: Tailwind CSS + shadcn/ui
- **State Management**: React hooks + context
- **Wallet Integration**: Freighter, XBull

### Infrastructure
- **CI/CD**: GitHub Actions
- **Containerization**: Docker & Docker Compose
- **Monitoring**: Prometheus/Grafana (planned)

---

## 📊 Current Status

**Phase**: M1 - Phase 1.1 (Environment & Project Setup)  
**Status**: ✅ **95% Complete**

### ✅ Completed
- Project structure initialized (Rust workspace with 5 crates)
- Docker Compose setup for PostgreSQL and Redis
- GitHub Actions CI/CD pipeline configured
- Documentation structure created
- Setup scripts and automation
- Planning files established (`task_plan.md`, `findings.md`, `progress.md`)

### 🔄 In Progress
- Manual Rust installation (see [Setup Guide](docs/development/SETUP.md))
- Soroban CLI installation (see [Setup Guide](docs/development/SETUP.md))

### 📋 Next Steps
1. Complete development environment setup
2. Begin Phase 1.2: SDEX Indexer Development
3. Design and implement database schema
4. Build Horizon API integration

See the full [Development Roadmap](Roadmap.md) for detailed milestones.

---

## 🚦 Getting Started

### Prerequisites
- Rust 1.75+ (installation instructions in [SETUP.md](docs/development/SETUP.md))
- Soroban CLI
- Docker & Docker Compose
- PostgreSQL 15+
- Node.js 18+ (for frontend development)

### Quick Start

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/StellarRoute.git
   cd StellarRoute
   ```

2. **Install Rust and Soroban CLI**
   Follow the detailed instructions in [docs/development/SETUP.md](docs/development/SETUP.md)

3. **Start local services**
   ```bash
   docker-compose up -d
   ```

4. **Build the project**
   ```bash
   cargo build
   ```

5. **Run tests**
   ```bash
   cargo test
   ```

For detailed setup instructions, see the [Development Setup Guide](docs/development/SETUP.md).

---

## 📦 Project Structure

```
StellarRoute/
├── crates/
│   ├── indexer/       # SDEX & Soroban indexing service
│   ├── api/           # REST API server
│   ├── routing/       # Routing engine & pathfinding
│   ├── contracts/     # Soroban smart contracts
│   └── sdk-rust/      # Rust SDK for developers
├── frontend/          # Web UI (React/Next.js) [planned]
├── sdk-js/            # JavaScript/TypeScript SDK [planned]
├── docs/              # Documentation
│   ├── architecture/  # Architecture documentation
│   ├── api/          # API reference
│   ├── development/  # Development guides
│   └── deployment/   # Deployment guides
├── scripts/          # Setup and utility scripts
├── docker-compose.yml # Local development services
├── Roadmap.md        # Detailed development roadmap
└── README.md         # This file
```

---

## 📈 Development Roadmap

StellarRoute is being developed in 5 major milestones:

| Milestone | Description | Status | Timeline |
|-----------|-------------|--------|----------|
| **M1** | Prototype Indexer & API (SDEX Only) | 🔄 In Progress | 6-8 weeks |
| **M2** | Soroban AMM Integration & Routing Engine | 🔴 Not Started | 8-10 weeks |
| **M3** | Smart Contracts & Soroban Deployment | 🔴 Not Started | 10-12 weeks |
| **M4** | Web UI & SDK Libraries | 🔴 Not Started | 10-12 weeks |
| **M5** | Audits, Documentation & Mainnet Launch | 🔴 Not Started | 8-10 weeks |

**Total Timeline**: ~10-12 months

See the complete [Development Roadmap](Roadmap.md) for detailed phase breakdowns and technical tasks.

---

## 🤝 Contributing

We welcome contributions from the community! StellarRoute is open-source and built for the Stellar ecosystem.

### How to Contribute
- Report bugs and issues
- Suggest new features
- Submit pull requests
- Improve documentation
- Help with testing

_(Detailed contribution guidelines coming soon)_

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🔗 Resources

- **Stellar Documentation**: https://developers.stellar.org
- **Soroban Documentation**: https://soroban.stellar.org
- **Horizon API Reference**: https://developers.stellar.org/api/horizon
- **Project Roadmap**: [Roadmap.md](Roadmap.md)
- **Development Setup**: [docs/development/SETUP.md](docs/development/SETUP.md)

---

## 📞 Support & Community

- **Issues**: [GitHub Issues](../../issues)
- **Discussions**: [GitHub Discussions](../../discussions)
- **Documentation**: [docs/](docs/)

---

## 🎯 Vision

Our goal is to create the most efficient, user-friendly, and developer-centric DEX aggregation platform on Stellar. By combining SDEX orderbook depth with Soroban AMM liquidity, we're building infrastructure that will help traders get the best prices while making it easy for developers to integrate sophisticated trading functionality into their applications.

**Built with ❤️ for the Stellar ecosystem**

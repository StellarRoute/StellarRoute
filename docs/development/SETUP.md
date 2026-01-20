# Development Setup Guide

This guide will help you set up your development environment for StellarRoute.

## Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Docker and Docker Compose
- Git

## Installation Steps

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Install Rust Toolchain for Soroban

```bash
rustup target add wasm32-unknown-unknown
```

### 3. Install Soroban CLI

**Option 1: Install via Cargo (Recommended)**
```bash
cargo install --locked soroban-cli
```

**Option 2: Install via Official Installer Script**
```bash
# For macOS/Linux
curl -sSfL https://github.com/stellar/soroban-tools/releases/latest/download/soroban-install.sh | sh
```

**Option 3: Manual Binary Download**
1. Visit [Soroban Tools Releases](https://github.com/stellar/soroban-tools/releases)
2. Download the appropriate binary for your platform
3. Extract and add to your PATH

**Verify Installation:**
```bash
soroban --version
```

**Note:** The Homebrew tap `stellar/soroban/soroban` is not currently available. Use one of the methods above instead.

### 4. Clone the Repository

```bash
git clone https://github.com/stellarroute/stellarroute.git
cd stellarroute
```

### 5. Start Local Services (Postgres & Redis)

```bash
docker-compose up -d
```

This will start:
- PostgreSQL on port 5432
- Redis on port 6379

### 6. Build the Project

```bash
cargo build
```

### 7. Run Tests

```bash
cargo test
```

## Environment Variables

Create a `.env` file in the project root:

```env
DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute
REDIS_URL=redis://localhost:6379
STELLAR_HORIZON_URL=https://horizon.stellar.org
SOROBAN_RPC_URL=https://soroban-rpc.testnet.stellar.org
```

## Next Steps

- See [Architecture Documentation](../architecture/README.md) for system design
- See [API Documentation](../api/README.md) for API reference
- See [Contract Documentation](../contracts/README.md) for smart contract details

## Troubleshooting

### Rust Installation Issues

If you encounter SSL errors during Rust installation, try:
1. Check your network connection
2. Use a VPN if behind a firewall
3. Install Rust manually from https://forge.rust-lang.org/infra/channel-layout.html

### Docker Issues

If Docker Compose fails:
- Ensure Docker Desktop is running (on macOS/Windows)
- Check that ports 5432 and 6379 are not already in use

### Build Issues

If cargo build fails:
- Update Rust: `rustup update stable`
- Clean build: `cargo clean && cargo build`
- Check that all prerequisites are installed

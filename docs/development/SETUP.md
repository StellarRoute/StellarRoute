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
- See [Wallet Integration Guide](./wallet-integration.md) for frontend wallet connection, signing, and testing patterns

## Troubleshooting

### Rust Installation Issues

#### SSL/TLS errors during `rustup` install

**Symptoms:** `curl: (60) SSL certificate problem` or `error: could not download file`

**Linux fix:**
```bash
sudo apt-get update && sudo apt-get install -y ca-certificates libssl-dev pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**macOS fix:**
```bash
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows fix:** Download the `rustup-init.exe` installer directly from https://rustup.rs and run it.

**Behind a corporate proxy:**
```bash
export HTTPS_PROXY=http://your-proxy:port
export HTTP_PROXY=http://your-proxy:port
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### `rustup` command not found after install

The installer modifies your shell profile but the current session may not have reloaded it.

```bash
source $HOME/.cargo/env          # bash / zsh
# or restart your terminal
```

#### Wrong Rust version

```bash
rustup update stable
rustup default stable
rustc --version   # should print 1.70 or higher
```

---

### Soroban CLI Troubleshooting

#### `cargo install --locked soroban-cli` fails with linker errors

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get install -y build-essential gcc libssl-dev pkg-config
cargo install --locked soroban-cli
```

**macOS:**
```bash
xcode-select --install         # installs Apple Command Line Tools
cargo install --locked soroban-cli
```

#### `wasm32-unknown-unknown` target missing

```bash
rustup target add wasm32-unknown-unknown
rustup target list --installed   # verify it appears
```

#### `soroban: command not found`

Cargo-installed binaries live in `~/.cargo/bin`. Make sure this directory is on your `PATH`:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc   # or ~/.zshrc
source ~/.bashrc
soroban --version
```

#### Soroban version mismatch

If you previously installed an older version, force a reinstall:

```bash
cargo install --locked --force soroban-cli
soroban --version
```

---

### Docker / PostgreSQL Issues

#### Docker daemon not running

**Linux:**
```bash
sudo systemctl start docker
sudo systemctl enable docker    # start on boot
```

**macOS/Windows:** Open Docker Desktop from your Applications folder.

#### Permission denied when running Docker commands (Linux)

```bash
sudo usermod -aG docker $USER
newgrp docker                 # apply without logging out
docker ps                     # verify
```

#### Port already in use (5432 or 6379)

Find and stop the conflicting process:

```bash
# Find the PID using the port
sudo lsof -i :5432
sudo lsof -i :6379

# Kill it (replace PID)
kill -9 <PID>

# Then start services again
docker-compose up -d
```

Alternatively, override the host port in `docker-compose.yml`:

```yaml
ports:
  - "5433:5432"   # use 5433 on the host
```

and update your `.env`:

```env
DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5433/stellarroute
```

#### `docker-compose up` fails — "network not found"

```bash
docker-compose down --volumes --remove-orphans
docker-compose up -d
```

#### PostgreSQL connection refused after `docker-compose up -d`

The container may still be starting. Wait for the health check to pass:

```bash
docker-compose ps   # STATUS should show "(healthy)"
# or wait explicitly
until docker-compose exec postgres pg_isready -U stellarroute; do sleep 1; done
```

#### Database connection failures from the application

1. Verify the container is healthy: `docker-compose ps`
2. Verify you can connect manually:
   ```bash
   psql postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute
   ```
3. Confirm your `.env` file exists in the project root and `DATABASE_URL` matches the credentials in `docker-compose.yml`.

---

### Cargo Build Errors

#### `error[E0463]: can't find crate for ...` / missing dependency

```bash
cargo clean
cargo build
```

If that fails, update your toolchain and retry:

```bash
rustup update stable
cargo build
```

#### Linker errors (`ld: library not found`)

**Linux:**
```bash
sudo apt-get install -y build-essential libssl-dev pkg-config
```

**macOS:**
```bash
xcode-select --install
```

#### `error: failed to run custom build command for openssl-sys`

```bash
# Linux
sudo apt-get install -y libssl-dev pkg-config

# macOS
brew install openssl
export PKG_CONFIG_PATH=$(brew --prefix openssl)/lib/pkgconfig
cargo build
```

#### Build succeeds but binary panics at startup — missing env vars

Make sure `.env` exists in the project root with all required variables:

```env
DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute
REDIS_URL=redis://localhost:6379
STELLAR_HORIZON_URL=https://horizon.stellar.org
SOROBAN_RPC_URL=https://soroban-rpc.testnet.stellar.org
```

---

### Environment Variable Configuration

The project reads environment variables at runtime. If you omit a required variable the service will refuse to start with a descriptive error.

| Variable | Default | Notes |
|---|---|---|
| `DATABASE_URL` | — | Required. Full PostgreSQL connection string. |
| `REDIS_URL` | — | Required. Redis connection string. |
| `STELLAR_HORIZON_URL` | `https://horizon.stellar.org` | Stellar public Horizon API |
| `SOROBAN_RPC_URL` | `https://soroban-rpc.testnet.stellar.org` | Soroban RPC endpoint |

---

## FAQ

**Q: Do I need a Stellar account to run the project locally?**  
A: No. The indexer pulls public data from Horizon. A Stellar account is only needed if you plan to submit transactions.

**Q: Why does `cargo test` fail with a database error?**  
A: Integration tests require a running PostgreSQL instance. Start it with `docker-compose up -d` and ensure `DATABASE_URL` is set.

**Q: The API returns 429 Too Many Requests during testing.**  
A: The server enforces 100 req/min per IP. In tests, use a different IP or increase the rate limit in the test configuration.

**Q: `docker-compose up` pulls images every time — how do I speed it up?**  
A: Images are cached locally after the first pull. Subsequent starts will be instant unless you run `docker-compose pull`.

**Q: How do I reset the database to a clean state?**  
```bash
docker-compose down -v          # removes volumes
docker-compose up -d            # fresh database
cargo run --bin stellarroute-indexer -- migrate   # re-run migrations
```

**Q: Where do I report a bug or ask for help?**  
Open an issue on GitHub or join the discussion in the repository's Discussions tab.

---

## Verification Steps

After completing setup, confirm everything works:

```bash
# 1. Rust toolchain
rustc --version        # e.g. rustc 1.77.0
cargo --version
soroban --version

# 2. Services healthy
docker-compose ps      # both postgres and redis show "(healthy)"

# 3. Build succeeds
cargo build

# 4. Unit + integration tests pass
cargo test

# 5. API starts
cargo run --bin stellarroute-api
# In another terminal:
curl http://localhost:8080/health   # should return {"status":"healthy",...}
```

---

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust Install Guide](https://www.rust-lang.org/tools/install)
- [Rustup documentation](https://rust-lang.github.io/rustup/)
- [Soroban documentation](https://developers.stellar.org/docs/smart-contracts)
- [Docker Compose reference](https://docs.docker.com/compose/)
- [SQLx documentation](https://docs.rs/sqlx)
- [Stellar Horizon API](https://developers.stellar.org/api/horizon)

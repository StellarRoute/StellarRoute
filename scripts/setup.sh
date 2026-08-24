#!/bin/bash
# StellarRoute Setup Script

set -e

echo "🚀 Setting up StellarRoute development environment..."

# Check Rust installation
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "✅ Rust is installed: $(rustc --version)"
fi

# Install WASM target for Soroban
echo "📦 Installing WASM target..."
rustup target add wasm32-unknown-unknown

# Check Soroban CLI installation
if ! command -v soroban &> /dev/null; then
    echo "⚠️  Soroban CLI is not installed."
    echo "   Install it with: cargo install --locked soroban-cli"
    echo "   Or visit: https://github.com/stellar/soroban-tools/releases"
else
    echo "✅ Soroban CLI is installed: $(soroban --version 2>&1 | head -n 1)"
fi

# Check Docker installation
if ! command -v docker &> /dev/null; then
    echo "⚠️  Docker is not installed. Please install Docker to run local services."
else
    echo "✅ Docker is installed: $(docker --version)"
fi

# Start Docker services
if command -v docker-compose &> /dev/null || docker compose version &> /dev/null; then
    echo "🐳 Starting Docker services (Postgres & Redis)..."
    docker-compose up -d || docker compose up -d
    echo "✅ Docker services started"
    
    # Wait for databases to be ready
    "$(dirname "$0")/wait-for-dbs.sh"
else
    echo "⚠️  Docker Compose is not available. Skipping service startup."
fi

# Build the project
echo "🔨 Building StellarRoute..."
cargo build

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "  1. Review docs/development/SETUP.md"
echo "  2. Create a .env file (see docs/development/SETUP.md)"
echo "  3. Run tests: cargo test"
echo ""

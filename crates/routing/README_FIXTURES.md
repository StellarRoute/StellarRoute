# Route Fixture Generator CLI

Deterministic routing graph fixture generator for integration tests and demos.

## Usage

```bash
cargo run --bin fixture-gen -- <COMMAND> [OPTIONS]
```

## Commands

### Minimal Market (Single-Hop)

Generate a minimal fixture with one SDEX offer and one AMM pool (XLM → USDC):

```bash
# Output to stdout
cargo run --bin fixture-gen -- minimal --format json

# Save to file
cargo run --bin fixture-gen -- minimal --format json --output fixtures/minimal.json

# Generate SQL
cargo run --bin fixture-gen -- minimal --format sql --output fixtures/minimal.sql
```

### Multi-Hop Market

Generate a multi-hop fixture with 2 SDEX offers and 2 AMM pools (XLM → USDC → EURC):

```bash
cargo run --bin fixture-gen -- multi-hop --format json --output fixtures/multi-hop.json
```

This scenario demonstrates:
- Direct AMM shortcut (XLM → EURC, 1 hop)
- 2-hop SDEX path (XLM → USDC → EURC)
- Route optimization across both venue types

### Thin Liquidity Market

Generate a fixture with very low reserves to test liquidity-floor exclusions:

```bash
cargo run --bin fixture-gen -- thin-liquidity --format json --output fixtures/thin-liquidity.json
```

### List Available Fixtures

```bash
cargo run --bin fixture-gen -- list
```

## Output Formats

### JSON Format

Portable JSON format with all fixtures represented:

```json
{
  "name": "minimal-market",
  "description": "...",
  "assets": [
    { "key": "native", "type": "native" },
    { "key": "USDC:GA5Z...", "type": "credit4", "code": "USDC", "issuer": "GA5Z..." }
  ],
  "sdex_offers": [...],
  "amm_pools": [...],
  "edges": [
    {
      "from": "native",
      "to": "USDC:GA5Z...",
      "venue_type": "sdex",
      "venue_ref": "sdex:1001",
      "liquidity": 100000000000,
      "price": 0.1,
      "fee_bps": 0
    }
  ]
}
```

**Suitable for:**
- Integration test fixtures in Rust (`crates/routing/tests/`)
- Frontend test mocks
- Documentation examples

### SQL Format

Insertable SQL script for populating test databases:

```sql
-- Fixture: minimal-market
-- This SQL script loads fixtures into the normalized_liquidity table

BEGIN TRANSACTION;

-- Assets
INSERT INTO assets (key, type) VALUES ('native', 'native') ON CONFLICT DO NOTHING;
INSERT INTO assets (key, type, code, issuer) VALUES ('USDC:GA5Z...', 'credit4', 'USDC', 'GA5Z...') ON CONFLICT DO NOTHING;

-- SDEX Offers
INSERT INTO sdex_offers (...) VALUES (...) ON CONFLICT DO NOTHING;

-- AMM Pools
INSERT INTO amm_pool_reserves (...) VALUES (...) ON CONFLICT DO NOTHING;

COMMIT;
```

**Suitable for:**
- Local database seeding
- CI integration test setup
- Performance testing

## Determinism

All fixtures are **deterministic and seed-based**:

- Same command always produces identical output
- No randomness or external dependencies
- Suitable for reproducible test scenarios
- Ideal for snapshot/baseline testing

## Usage in Tests

### Rust Integration Tests

```rust
use stellarroute_routing::fixtures::FixtureBuilder;

#[test]
fn test_routing_with_minimal_market() {
    let edges = FixtureBuilder::minimal_market().build_edges();
    let pathfinder = Pathfinder::new(config);
    
    let paths = pathfinder.find_paths("native", "USDC:...", &edges, 100_000_000, &policy)?;
    assert!(!paths.is_empty());
}
```

### Frontend E2E Tests

Mock API responses with generated JSON fixtures:

```typescript
await page.route('/api/v1/quote/**', async (route) => {
  const fixture = require('./fixtures/minimal.json');
  await route.fulfill({
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify(fixture.edges)
  });
});
```

## Sample Scenarios

### Scenario 1: Single-Hop SDEX Route

```bash
cargo run --bin fixture-gen -- minimal --format json | jq '.edges[] | select(.venue_type == "sdex")'
```

### Scenario 2: Single-Hop AMM Route

```bash
cargo run --bin fixture-gen -- minimal --format json | jq '.edges[] | select(.venue_type == "amm")'
```

### Scenario 3: Multi-Hop Path Optimization

```bash
cargo run --bin fixture-gen -- multi-hop --format json --output /tmp/multi.json
cargo test --test graph_fixtures_integration -- --nocapture
```

## Adding New Fixtures

To add a new fixture scenario, extend `FixtureBuilder` in `crates/routing/src/fixtures.rs`:

```rust
impl FixtureBuilder {
    pub fn my_custom_scenario() -> Self {
        let xlm = FixtureAsset::native();
        let usdc = FixtureAsset::credit4("USDC", "GA5Z...");
        
        Self::new()
            .with_asset(xlm.clone())
            .with_asset(usdc.clone())
            .with_sdex_offer(FixtureSdexOffer { ... })
            .with_amm_pool(FixtureAmmPool { ... })
    }
}
```

Then update the CLI subcommand in `fixture_gen.rs` to support it:

```rust
Commands::MyScenario { format, output } => {
    generate_fixture("my-scenario", &format, output)?;
}
```

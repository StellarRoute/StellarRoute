//! Reusable in-memory DB fixtures for routing graph integration tests.
//!
//! Provides a [`FixtureBuilder`] that generates deterministic, minimal-viable
//! market graphs covering both SDEX orderbook and AMM pool representations.
//! All fixtures are self-contained and require no external network or database.
//!
//! # Example
//! ```rust
//! use stellarroute_routing::fixtures::FixtureBuilder;
//!
//! let edges = FixtureBuilder::minimal_market().build_edges();
//! assert!(!edges.is_empty());
//! ```

use crate::normalization::{AmmReserveInput, SdexLevelInput};
use crate::pathfinder::LiquidityEdge;

/// A single asset in the fixture graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureAsset {
    /// Canonical key used as node identifier in the routing graph.
    /// Format: `"native"` or `"CODE:ISSUER"`.
    pub key: String,
    pub asset_type: AssetType,
}

/// Asset type mirroring the DB schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetType {
    Native,
    CreditAlphanum4 { code: String, issuer: String },
    CreditAlphanum12 { code: String, issuer: String },
}

impl FixtureAsset {
    pub fn native() -> Self {
        Self {
            key: "native".to_string(),
            asset_type: AssetType::Native,
        }
    }

    pub fn credit4(code: &str, issuer: &str) -> Self {
        Self {
            key: format!("{}:{}", code, issuer),
            asset_type: AssetType::CreditAlphanum4 {
                code: code.to_string(),
                issuer: issuer.to_string(),
            },
        }
    }

    pub fn credit12(code: &str, issuer: &str) -> Self {
        Self {
            key: format!("{}:{}", code, issuer),
            asset_type: AssetType::CreditAlphanum12 {
                code: code.to_string(),
                issuer: issuer.to_string(),
            },
        }
    }
}

/// A fixture SDEX offer row (mirrors `sdex_offers` table).
#[derive(Debug, Clone)]
pub struct FixtureSdexOffer {
    pub offer_id: i64,
    pub seller: String,
    pub selling_asset: FixtureAsset,
    pub buying_asset: FixtureAsset,
    /// Decimal string, e.g. `"1.0500000"`
    pub amount: String,
    /// Decimal string, e.g. `"0.1000000"`
    pub price: String,
    pub last_modified_ledger: i64,
}

/// A fixture AMM pool row (mirrors `amm_pool_reserves` table).
#[derive(Debug, Clone)]
pub struct FixtureAmmPool {
    pub pool_address: String,
    pub selling_asset: FixtureAsset,
    pub buying_asset: FixtureAsset,
    /// Decimal string
    pub reserve_selling: String,
    /// Decimal string
    pub reserve_buying: String,
    pub fee_bps: u32,
    pub last_updated_ledger: i64,
}

/// Builder for deterministic routing graph fixtures.
///
/// Supports both SDEX and AMM representations and converts them to
/// [`LiquidityEdge`] slices ready for the pathfinder.
#[derive(Debug, Default)]
pub struct FixtureBuilder {
    assets: Vec<FixtureAsset>,
    sdex_offers: Vec<FixtureSdexOffer>,
    amm_pools: Vec<FixtureAmmPool>,
}

impl FixtureBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Preset markets ────────────────────────────────────────────────────────

    /// Minimal viable market: XLM → USDC via one SDEX offer and one AMM pool.
    /// Sufficient to test single-hop routing with both venue types.
    pub fn minimal_market() -> Self {
        let xlm = FixtureAsset::native();
        let usdc = FixtureAsset::credit4(
            "USDC",
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
        );

        Self::new()
            .with_asset(xlm.clone())
            .with_asset(usdc.clone())
            .with_sdex_offer(FixtureSdexOffer {
                offer_id: 1001,
                seller: "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN".to_string(),
                selling_asset: xlm.clone(),
                buying_asset: usdc.clone(),
                amount: "10000.0000000".to_string(),
                price: "0.1000000".to_string(),
                last_modified_ledger: 50_000_000,
            })
            .with_amm_pool(FixtureAmmPool {
                pool_address: "CAMMPOOL1XLMUSDC000000000000000000000000000000000000000001"
                    .to_string(),
                selling_asset: xlm.clone(),
                buying_asset: usdc.clone(),
                reserve_selling: "500000.0000000".to_string(),
                reserve_buying: "50000.0000000".to_string(),
                fee_bps: 30,
                last_updated_ledger: 50_000_000,
            })
    }

    /// Multi-hop market: XLM → USDC → EURC, covering a 2-hop SDEX path and
    /// a direct AMM shortcut. Used to verify multi-hop route discovery.
    pub fn multi_hop_market() -> Self {
        let xlm = FixtureAsset::native();
        let usdc = FixtureAsset::credit4(
            "USDC",
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
        );
        let eurc = FixtureAsset::credit4(
            "EURC",
            "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP",
        );

        Self::new()
            .with_asset(xlm.clone())
            .with_asset(usdc.clone())
            .with_asset(eurc.clone())
            // Hop 1: XLM → USDC via SDEX
            .with_sdex_offer(FixtureSdexOffer {
                offer_id: 2001,
                seller: "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN".to_string(),
                selling_asset: xlm.clone(),
                buying_asset: usdc.clone(),
                amount: "20000.0000000".to_string(),
                price: "0.1000000".to_string(),
                last_modified_ledger: 50_000_001,
            })
            // Hop 2: USDC → EURC via SDEX
            .with_sdex_offer(FixtureSdexOffer {
                offer_id: 2002,
                seller: "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN".to_string(),
                selling_asset: usdc.clone(),
                buying_asset: eurc.clone(),
                amount: "5000.0000000".to_string(),
                price: "0.9200000".to_string(),
                last_modified_ledger: 50_000_001,
            })
            // Direct shortcut: XLM → EURC via AMM
            .with_amm_pool(FixtureAmmPool {
                pool_address: "CAMMPOOL2XLMEURC000000000000000000000000000000000000000002"
                    .to_string(),
                selling_asset: xlm.clone(),
                buying_asset: eurc.clone(),
                reserve_selling: "300000.0000000".to_string(),
                reserve_buying: "27600.0000000".to_string(),
                fee_bps: 30,
                last_updated_ledger: 50_000_001,
            })
            // AMM: USDC → EURC (alternative to SDEX hop 2)
            .with_amm_pool(FixtureAmmPool {
                pool_address: "CAMMPOOL3USDCEURC000000000000000000000000000000000000000003"
                    .to_string(),
                selling_asset: usdc.clone(),
                buying_asset: eurc.clone(),
                reserve_selling: "200000.0000000".to_string(),
                reserve_buying: "184000.0000000".to_string(),
                fee_bps: 25,
                last_updated_ledger: 50_000_001,
            })
    }

    /// Thin liquidity market: very low reserves to test liquidity-floor exclusions.
    pub fn thin_liquidity_market() -> Self {
        let xlm = FixtureAsset::native();
        let usdc = FixtureAsset::credit4(
            "USDC",
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
        );

        Self::new()
            .with_asset(xlm.clone())
            .with_asset(usdc.clone())
            .with_sdex_offer(FixtureSdexOffer {
                offer_id: 3001,
                seller: "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN".to_string(),
                selling_asset: xlm.clone(),
                buying_asset: usdc.clone(),
                amount: "0.0100000".to_string(),
                price: "0.1000000".to_string(),
                last_modified_ledger: 50_000_002,
            })
            .with_amm_pool(FixtureAmmPool {
                pool_address: "CAMMPOOL4THIN0000000000000000000000000000000000000000000004"
                    .to_string(),
                selling_asset: xlm.clone(),
                buying_asset: usdc.clone(),
                reserve_selling: "0.1000000".to_string(),
                reserve_buying: "0.0100000".to_string(),
                fee_bps: 30,
                last_updated_ledger: 50_000_002,
            })
    }

    // ── Builder methods ───────────────────────────────────────────────────────

    pub fn with_asset(mut self, asset: FixtureAsset) -> Self {
        if !self.assets.iter().any(|a| a.key == asset.key) {
            self.assets.push(asset);
        }
        self
    }

    pub fn with_sdex_offer(mut self, offer: FixtureSdexOffer) -> Self {
        self.sdex_offers.push(offer);
        self
    }

    pub fn with_amm_pool(mut self, pool: FixtureAmmPool) -> Self {
        self.amm_pools.push(pool);
        self
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn assets(&self) -> &[FixtureAsset] {
        &self.assets
    }

    pub fn sdex_offers(&self) -> &[FixtureSdexOffer] {
        &self.sdex_offers
    }

    pub fn amm_pools(&self) -> &[FixtureAmmPool] {
        &self.amm_pools
    }

    // ── Conversion helpers ────────────────────────────────────────────────────

    /// Convert SDEX offers to [`SdexLevelInput`] for the normalization layer.
    pub fn sdex_level_inputs(&self) -> Vec<SdexLevelInput> {
        self.sdex_offers
            .iter()
            .map(|o| SdexLevelInput {
                offer_id: o.offer_id,
                price: o.price.clone(),
                amount: o.amount.clone(),
            })
            .collect()
    }

    /// Convert AMM pools to [`AmmReserveInput`] for the normalization layer.
    pub fn amm_reserve_inputs(&self) -> Vec<AmmReserveInput> {
        self.amm_pools
            .iter()
            .map(|p| AmmReserveInput {
                pool_address: p.pool_address.clone(),
                reserve_selling: p.reserve_selling.clone(),
                reserve_buying: p.reserve_buying.clone(),
                fee_bps: p.fee_bps,
            })
            .collect()
    }

    /// Build [`LiquidityEdge`] slices ready for the pathfinder.
    ///
    /// Each SDEX offer becomes one directed edge; each AMM pool becomes two
    /// directed edges (both directions) to reflect the symmetric nature of
    /// constant-product pools.
    pub fn build_edges(&self) -> Vec<LiquidityEdge> {
        let mut edges = Vec::new();

        for offer in &self.sdex_offers {
            let price: f64 = offer.price.parse().unwrap_or(1.0);
            let liquidity = parse_amount_to_e7(&offer.amount);
            edges.push(LiquidityEdge {
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
                from: offer.selling_asset.key.clone(),
                to: offer.buying_asset.key.clone(),
                venue_type: "sdex".to_string(),
                venue_ref: format!("sdex:{}", offer.offer_id),
                liquidity,
                price,
                fee_bps: 0,
                anomaly_score: 0.0,
                anomaly_reasons: Vec::new(),
            });
        }

        for pool in &self.amm_pools {
            let reserve_selling = parse_amount_to_e7(&pool.reserve_selling);
            let reserve_buying = parse_amount_to_e7(&pool.reserve_buying);
            let price_fwd = if reserve_selling > 0 {
                reserve_buying as f64 / reserve_selling as f64
            } else {
                1.0
            };
            let price_rev = if reserve_buying > 0 {
                reserve_selling as f64 / reserve_buying as f64
            } else {
                1.0
            };

            // Forward direction
            edges.push(LiquidityEdge {
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
                from: pool.selling_asset.key.clone(),
                to: pool.buying_asset.key.clone(),
                venue_type: "amm".to_string(),
                venue_ref: pool.pool_address.clone(),
                liquidity: reserve_selling,
                price: price_fwd,
                fee_bps: pool.fee_bps,
                anomaly_score: 0.0,
                anomaly_reasons: Vec::new(),
            });

            // Reverse direction (AMM pools are symmetric)
            edges.push(LiquidityEdge {
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
                from: pool.buying_asset.key.clone(),
                to: pool.selling_asset.key.clone(),
                venue_type: "amm".to_string(),
                venue_ref: pool.pool_address.clone(),
                liquidity: reserve_buying,
                price: price_rev,
                fee_bps: pool.fee_bps,
                anomaly_score: 0.0,
                anomaly_reasons: Vec::new(),
            });
        }

        edges
    }
}

/// Parse a decimal string like `"10000.0000000"` into e7-scaled i128.
fn parse_amount_to_e7(s: &str) -> i128 {
    let trimmed = s.trim();
    let mut parts = trimmed.splitn(2, '.');
    let int_part: i128 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let frac_str = parts.next().unwrap_or("0000000");
    let frac_padded = format!("{:0<7}", &frac_str[..frac_str.len().min(7)]);
    let frac_part: i128 = frac_padded.parse().unwrap_or(0);
    int_part * 10_000_000 + frac_part
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_market_has_both_venue_types() {
        let fb = FixtureBuilder::minimal_market();
        assert!(fb.sdex_offers().iter().any(|o| o.offer_id == 1001));
        assert!(!fb.amm_pools().is_empty());
    }

    #[test]
    fn build_edges_produces_directed_edges() {
        let edges = FixtureBuilder::minimal_market().build_edges();
        // 1 SDEX offer + 1 AMM pool (2 directions) = 3 edges
        assert_eq!(edges.len(), 3);
        assert!(edges.iter().any(|e| e.venue_type == "sdex"));
        assert!(edges.iter().any(|e| e.venue_type == "amm"));
    }

    #[test]
    fn multi_hop_market_has_three_assets() {
        let fb = FixtureBuilder::multi_hop_market();
        assert_eq!(fb.assets().len(), 3);
    }

    #[test]
    fn amm_pool_edges_are_bidirectional() {
        let fb = FixtureBuilder::minimal_market();
        let edges = fb.build_edges();
        let amm_edges: Vec<_> = edges.iter().filter(|e| e.venue_type == "amm").collect();
        // One pool → two directed edges
        assert_eq!(amm_edges.len(), 2);
        let froms: Vec<_> = amm_edges.iter().map(|e| e.from.as_str()).collect();
        assert!(froms.contains(&"native"));
        let usdc_key = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        assert!(froms.contains(&usdc_key));
    }

    #[test]
    fn sdex_level_inputs_round_trip() {
        let fb = FixtureBuilder::minimal_market();
        let inputs = fb.sdex_level_inputs();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].offer_id, 1001);
        assert_eq!(inputs[0].price, "0.1000000");
    }

    #[test]
    fn parse_amount_to_e7_precision() {
        assert_eq!(parse_amount_to_e7("1.0000000"), 10_000_000);
        assert_eq!(parse_amount_to_e7("10000.0000000"), 100_000_000_000);
        assert_eq!(parse_amount_to_e7("0.0100000"), 100_000);
    }
}

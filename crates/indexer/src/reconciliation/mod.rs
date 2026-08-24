//! Stateful market-data reconciliation engine
//!
//! Detects and repairs drift between Horizon-indexed orderbook data and Soroban RPC pool state.
//!
//! # Architecture
//!
//! The reconciliation engine performs five types of consistency checks:
//!
//! 1. **Asset Mapping**: Ensures all asset references are valid
//! 2. **Price Divergence**: Detects when SDEX and AMM prices diverge beyond thresholds
//! 3. **Ledger Alignment**: Monitors ledger sequence drift between venues
//! 4. **Liquidity Anomalies**: Detects sudden reserve/amount changes (possible refills or drains)
//! 5. **Data Staleness**: Flags updates that exceed configured age thresholds
//!
//! # Repair Workflow
//!
//! For each detected issue, the engine triggers appropriate repairs:
//!
//! - **Info**: No action needed
//! - **Warning**: Log event and emit metrics
//! - **Critical**: Trigger automatic repair (refetch/invalidate) and alert operator
//!
//! # Metrics & Alerts
//!
//! All checks and repairs are recorded with:
//! - `reconciliation_checks`: Individual check results
//! - `drift_events`: Time-series metrics for monitoring
//! - `repair_actions`: Attempted fixes and success rates
//! - `reconciliation_runs`: Cycle summaries for dashboards
//!
//! # Example Usage
//!
//! ```ignore
//! let engine = ReconciliationEngine::new(db).await?;
//! let run = engine.run_reconciliation_cycle().await?;
//!
//! println!("Checks: {}/{} passed", run.checks_passed, run.checks_executed);
//! println!("Critical issues: {}", run.critical_drift_events);
//! println!("Repairs attempted: {}/{}", run.successful_repairs, run.total_repairs_attempted);
//! ```

pub mod backfill;
pub mod consistency;
pub mod engine;
pub mod metrics;
pub mod repair;

pub use backfill::{BackfillCheckpoint, BackfillManager, BackfillStatus};
pub use consistency::{CheckThresholds, CheckType, ConsistencyCheckResult, DriftSeverity};
pub use engine::{ReconciliationEngine, ReconciliationRun};
pub use metrics::{DriftMetrics, MetricsSnapshot, ReconciliationMetrics};
pub use repair::{RepairAction, RepairActionType, RepairWorkflow};

//! Deterministic quote replay system for post-incident analysis.
//!
//! # Overview
//!
//! This module provides a purely additive replay pipeline that captures quote
//! computation inputs, stores them as redacted artifacts, and allows engineers
//! to deterministically reproduce any historical routing decision.
//!
//! # Components
//!
//! - [`artifact`]: `ReplayArtifact` data types and PostgreSQL persistence
//! - [`redactor`]: Replaces sensitive fields (`asset_issuer`) with `[REDACTED]`
//! - [`engine`]: Pure deterministic replay of route selection from a stored artifact
//! - [`diff`]: Field-level comparison of original vs replayed outputs
//! - [`capture`]: Non-blocking capture hook for the live quote pipeline

pub mod artifact;
pub mod capture;
pub mod diff;
pub mod engine;
pub mod redactor;

pub use artifact::{ArtifactSummary, HealthConfigSnapshot, LiquidityCandidate, ReplayArtifact};
pub use capture::CaptureHook;
pub use diff::{DiffEngine, DiffReport, FieldDivergence};
pub use engine::{ReplayEngine, ReplayOutput};
pub use redactor::Redactor;

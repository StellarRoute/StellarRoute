//! Distributed route-computation worker pool
//!
//! Provides a queue-based architecture for handling route computation tasks with:
//! - Durable, priority-aware job queue (WFQ scheduling)
//! - Adaptive request prioritization based on amount and request type
//! - Starvation prevention via weighted virtual clock
//! - Job deduplication
//! - Backpressure protection
//! - Configurable retry logic

pub mod backpressure;
pub mod deduplication;
pub mod job;
pub mod pool;
pub mod priority;
pub mod queue;
pub mod retry;

pub use backpressure::BackpressurePolicy;
pub use job::{RouteComputationJob, RouteComputationTaskPayload};
pub use pool::{PoolMetricsSnapshot, RouteWorkerPool, WorkerPoolConfig};
pub use priority::{PriorityClassifier, PriorityConfig, RequestPriority};
pub use queue::{JobQueue, QueueStats};

//! Distributed route-computation worker pool
//!
//! Provides a queue-based architecture for handling route computation tasks with:
//! - Durable job queue
//! - Job deduplication
//! - Backpressure protection
//! - Configurable retry logic

pub mod backpressure;
pub mod deduplication;
pub mod job;
pub mod pool;
pub mod queue;
pub mod retry;

pub use backpressure::BackpressurePolicy;
pub use job::{RouteComputationJob, RouteComputationTaskPayload};
pub use pool::{RouteWorkerPool, WorkerPoolConfig};
pub use queue::JobQueue;

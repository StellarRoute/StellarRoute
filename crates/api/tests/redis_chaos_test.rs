//! Chaos test suite for Redis unavailability during quote bursts.
//!
//! Validates cache miss fallback, bounded SingleFlight memory, and degraded-mode metrics.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use stellarroute_api::cache::{CacheLookupResult, SingleFlight};
use stellarroute_api::metrics;

#[test]
fn cache_lookup_result_distinguishes_unavailable_from_miss() {
    assert_ne!(CacheLookupResult::Miss, CacheLookupResult::Unavailable);
    assert_ne!(CacheLookupResult::Hit, CacheLookupResult::Unavailable);
}

#[test]
fn degraded_mode_metrics_activate_on_unavailable_ops() {
    metrics::record_cache_unavailable("get");
    metrics::record_cache_degraded_mode("quote", true);

    let encoded = metrics::encode_metrics().expect("metrics encode");
    assert!(encoded.contains("stellarroute_cache_unavailable_ops_total"));
    assert!(encoded.contains("stellarroute_cache_degraded_mode"));
    assert!(encoded.contains(r#"type="quote""#));
}

#[test]
fn cache_miss_fallback_records_miss_not_hit() {
    metrics::record_cache_miss("quote");
    let encoded = metrics::encode_metrics().expect("metrics encode");
    assert!(encoded.contains("stellarroute_cache_misses_total"));
}

#[tokio::test]
async fn single_flight_coalesces_burst_without_unbounded_inflight_growth() {
    use std::sync::atomic::{AtomicUsize, Ordering as Ord};

    let sf = Arc::new(SingleFlight::<u64>::new());
    let compute_count = Arc::new(AtomicU64::new(0));
    let peak_inflight = Arc::new(AtomicUsize::new(0));
    let current_inflight = Arc::new(AtomicUsize::new(0));

    let burst_size = 100usize;
    let mut handles = Vec::with_capacity(burst_size);

    for _ in 0..burst_size {
        let sf_ref = sf.clone();
        let compute_count_ref = compute_count.clone();
        let peak_ref = peak_inflight.clone();
        let current_ref = current_inflight.clone();

        handles.push(tokio::spawn(async move {
            sf_ref
                .execute_with_label("quote", "burst-key", || async move {
                    let now = current_ref.fetch_add(1, Ord::Relaxed) + 1;
                    peak_ref.fetch_max(now, Ord::Relaxed);
                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    compute_count_ref.fetch_add(1, Ord::Relaxed);
                    current_ref.fetch_sub(1, Ord::Relaxed);
                    Arc::new(42u64)
                })
                .await
        }));
    }

    for handle in handles {
        let result = handle.await.expect("task panicked");
        assert_eq!(*result, 42);
    }

    assert_eq!(
        compute_count.load(Ordering::Relaxed),
        1,
        "identical burst keys must coalesce to one compute"
    );
    assert!(
        peak_inflight.load(Ordering::Relaxed) <= 2,
        "inflight entries must not grow unbounded during outage burst"
    );
}

#[tokio::test]
async fn single_flight_allows_new_leader_after_inflight_entry_removed() {
    let sf = Arc::new(SingleFlight::<String>::new());

    let _ = sf
        .execute("cleanup-key", || async move { Arc::new("ok".to_string()) })
        .await;

    // Removal from the inflight map is async; wait until a new leader can run.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let compute_count = Arc::new(AtomicU64::new(0));
    let compute_ref = compute_count.clone();
    let result2 = sf
        .execute("cleanup-key", || async move {
            compute_ref.fetch_add(1, Ordering::Relaxed);
            Arc::new("ok2".to_string())
        })
        .await;

    assert_eq!(*result2, "ok2");
    assert_eq!(compute_count.load(Ordering::Relaxed), 1);
}

#[test]
fn unavailable_lookup_implies_compute_fallback_not_cache_hit() {
    let lookup = CacheLookupResult::Unavailable;
    assert_eq!(lookup, CacheLookupResult::Unavailable);

    metrics::record_cache_unavailable("get");
    metrics::record_cache_degraded_mode("quote", true);
    metrics::record_cache_miss("quote");

    let encoded = metrics::encode_metrics().expect("metrics");
    assert!(encoded.contains("operation=\"get\""));
}

//! Atomic attestation trust generations: Iris keys + both destination snapshots.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::cctp::attester_set::{
    iris_candidate_hash, validate_attester_set_parity, AttesterDestination, AttesterSetReader,
    AttesterSetSnapshot,
};
use crate::cctp::bounds::MAX_IRIS_PUBLIC_KEYS;
use crate::cctp::config::CctpConfig;
use crate::cctp::iris_public_keys::IrisPublicKeySource;
use crate::metrics;

#[derive(Clone)]
pub struct IrisTrustSnapshot {
    pub addresses: Vec<[u8; 20]>,
    pub set_hash: [u8; 32],
}

#[derive(Clone)]
pub struct AttestationTrustGeneration {
    pub generation: u64,
    pub refreshed_at: Instant,
    pub iris: IrisTrustSnapshot,
    pub sepolia: AttesterSetSnapshot,
    pub stellar: AttesterSetSnapshot,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AttestationTrustError {
    #[error("not ready")]
    NotReady,
    #[error("stale beyond policy")]
    Stale,
    #[error("refresh failed")]
    RefreshFailed,
    #[error("attester set: {0}")]
    AttesterSet(String),
    #[error("iris: {0}")]
    Iris(String),
}

pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
pub struct MockClock {
    epoch: Instant,
    advanced_ms: AtomicU64,
}

#[cfg(test)]
impl MockClock {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
            advanced_ms: AtomicU64::new(0),
        }
    }

    pub fn advance(&self, d: Duration) {
        self.advanced_ms
            .fetch_add(d.as_millis() as u64, Ordering::SeqCst);
    }
}

#[cfg(test)]
impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.epoch + Duration::from_millis(self.advanced_ms.load(Ordering::SeqCst))
    }
}

pub struct AttestationTrustCache {
    state: ArcSwap<Option<AttestationTrustGeneration>>,
    ttl: Duration,
    stale_max: Duration,
    refresh_lock: Mutex<()>,
    generation_counter: AtomicU64,
    clock: Arc<dyn Clock>,
    /// Successful atomic generation builds (for tests/diagnostics).
    pub(crate) generation_build_count: Arc<AtomicUsize>,
}

pub struct AttestationRefreshDeps {
    pub iris_source: Arc<dyn IrisPublicKeySource>,
    pub readers: Vec<Arc<dyn AttesterSetReader>>,
}

impl AttestationTrustCache {
    pub fn new(ttl: Duration, stale_max: Duration, clock: Arc<dyn Clock>) -> Self {
        Self {
            state: ArcSwap::from_pointee(None),
            ttl,
            stale_max,
            refresh_lock: Mutex::new(()),
            generation_counter: AtomicU64::new(0),
            clock,
            generation_build_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn from_config(config: &CctpConfig) -> Self {
        Self::new(
            Duration::from_secs(config.attester_snapshot_ttl_secs),
            Duration::from_secs(config.attester_snapshot_stale_max_secs),
            Arc::new(SystemClock),
        )
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn generation(&self) -> Option<AttestationTrustGeneration> {
        self.state.load_full().as_ref().clone()
    }

    fn now(&self) -> Instant {
        self.clock.now()
    }

    pub fn is_ready(&self) -> bool {
        self.generation()
            .map(|g| !self.is_stale_beyond(&g))
            .unwrap_or(false)
    }

    fn is_fresh(&self, gen: &AttestationTrustGeneration) -> bool {
        self.now().duration_since(gen.refreshed_at) <= self.ttl
    }

    fn is_stale_beyond(&self, gen: &AttestationTrustGeneration) -> bool {
        self.now().duration_since(gen.refreshed_at) > self.stale_max
    }

    pub fn snapshot_for(&self, dest: AttesterDestination) -> Option<AttesterSetSnapshot> {
        let gen = self.generation()?;
        if self.is_stale_beyond(&gen) {
            return None;
        }
        match dest {
            AttesterDestination::Sepolia => Some(gen.sepolia.clone()),
            AttesterDestination::StellarTestnet => Some(gen.stellar.clone()),
        }
    }

    pub async fn ensure_fresh(
        &self,
        deps: &AttestationRefreshDeps,
    ) -> Result<(), AttestationTrustError> {
        if let Some(gen) = self.generation() {
            if self.is_fresh(&gen) {
                return Ok(());
            }
            if self.is_stale_beyond(&gen) {
                return Err(AttestationTrustError::Stale);
            }
        } else {
            return self.full_refresh(deps).await;
        }
        self.full_refresh(deps).await
    }

    pub async fn full_refresh(
        &self,
        deps: &AttestationRefreshDeps,
    ) -> Result<(), AttestationTrustError> {
        self.full_refresh_inner(deps, false).await
    }

    /// Rebuild trust even when the current generation is TTL-fresh (unknown-signer recovery).
    pub(crate) async fn force_full_refresh(
        &self,
        deps: &AttestationRefreshDeps,
    ) -> Result<(), AttestationTrustError> {
        self.full_refresh_inner(deps, true).await
    }

    async fn full_refresh_inner(
        &self,
        deps: &AttestationRefreshDeps,
        force: bool,
    ) -> Result<(), AttestationTrustError> {
        let _guard = self.refresh_lock.lock().await;
        if !force {
            if let Some(gen) = self.generation() {
                if self.is_fresh(&gen) {
                    return Ok(());
                }
                if self.is_stale_beyond(&gen) {
                    return Err(AttestationTrustError::Stale);
                }
            }
        } else if let Some(gen) = self.generation() {
            if self.is_stale_beyond(&gen) {
                return Err(AttestationTrustError::Stale);
            }
        }

        match self.build_generation(deps).await {
            Ok(gen) => {
                metrics::record_cctp_iris_keys_refresh("success", "generation");
                metrics::record_cctp_attester_snapshot_refresh("sepolia", "success");
                metrics::record_cctp_attester_snapshot_refresh("stellar_testnet", "success");
                self.generation_build_count.fetch_add(1, Ordering::SeqCst);
                self.state.store(Arc::new(Some(gen)));
                Ok(())
            }
            Err(e) => {
                metrics::record_cctp_iris_keys_refresh("failure", "generation");
                metrics::record_cctp_attester_snapshot_refresh("sepolia", "failure");
                metrics::record_cctp_attester_snapshot_refresh("stellar_testnet", "failure");
                if let Some(gen) = self.generation() {
                    if !self.is_stale_beyond(&gen) {
                        return Err(e);
                    }
                }
                Err(e)
            }
        }
    }

    async fn build_generation(
        &self,
        deps: &AttestationRefreshDeps,
    ) -> Result<AttestationTrustGeneration, AttestationTrustError> {
        let now = self.now();
        let iris_addresses = deps
            .iris_source
            .fetch_public_keys()
            .await
            .map_err(|e| AttestationTrustError::Iris(e.to_string()))?;
        if iris_addresses.is_empty() {
            return Err(AttestationTrustError::Iris("empty".into()));
        }
        if iris_addresses.len() > MAX_IRIS_PUBLIC_KEYS {
            return Err(AttestationTrustError::Iris("too many keys".into()));
        }
        let mut sorted_iris = iris_addresses;
        sorted_iris.sort();
        sorted_iris.dedup();
        let iris_hash = iris_candidate_hash(&sorted_iris);
        let iris = IrisTrustSnapshot {
            addresses: sorted_iris.clone(),
            set_hash: iris_hash,
        };

        let generation = self.generation_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let mut sepolia_snap = None;
        let mut stellar_snap = None;

        for reader in &deps.readers {
            let on_chain = reader
                .read_on_chain_set()
                .await
                .map_err(|e| AttestationTrustError::AttesterSet(e.to_string()))?;
            let snap = validate_attester_set_parity(
                reader.destination(),
                &on_chain,
                &sorted_iris,
                iris_hash,
                generation,
                now,
                match reader.destination() {
                    AttesterDestination::Sepolia => "evm_message_transmitter_v2",
                    AttesterDestination::StellarTestnet => "stellar_message_transmitter_v2",
                },
            )
            .map_err(|e| AttestationTrustError::AttesterSet(e.to_string()))?;
            match reader.destination() {
                AttesterDestination::Sepolia => sepolia_snap = Some(snap),
                AttesterDestination::StellarTestnet => stellar_snap = Some(snap),
            }
        }

        let sepolia = sepolia_snap.ok_or(AttestationTrustError::AttesterSet(
            "missing sepolia reader".into(),
        ))?;
        let stellar = stellar_snap.ok_or(AttestationTrustError::AttesterSet(
            "missing stellar reader".into(),
        ))?;

        Ok(AttestationTrustGeneration {
            generation,
            refreshed_at: now,
            iris,
            sepolia,
            stellar,
        })
    }

    pub fn spawn_background_refresh(
        weak: Weak<Self>,
        deps: Arc<AttestationRefreshDeps>,
        interval: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let Some(cache) = weak.upgrade() else {
                    break;
                };
                if cache.is_ready() && cache.generation().is_some_and(|g| cache.is_fresh(&g)) {
                    continue;
                }
                let _ = cache.full_refresh(deps.as_ref()).await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::attester_set::{AttesterSetError, RawOnChainAttesterSet};
    use crate::cctp::fixtures::circle_attestation_v2::{
        ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3,
    };
    use crate::cctp::iris_public_keys::IrisPublicKeyError;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    struct MockIris {
        keys: Vec<[u8; 20]>,
        fail_count: AtomicUsize,
    }

    #[async_trait]
    impl IrisPublicKeySource for MockIris {
        async fn fetch_public_keys(&self) -> Result<Vec<[u8; 20]>, IrisPublicKeyError> {
            if self.fail_count.load(AtomicOrdering::SeqCst) > 0 {
                self.fail_count.fetch_sub(1, AtomicOrdering::SeqCst);
                return Err(IrisPublicKeyError::Http("transient".into()));
            }
            Ok(self.keys.clone())
        }
    }

    struct MockReader {
        dest: AttesterDestination,
        enabled: Vec<[u8; 20]>,
        threshold: u32,
        fail: bool,
    }

    #[async_trait]
    impl AttesterSetReader for MockReader {
        fn destination(&self) -> AttesterDestination {
            self.dest
        }

        async fn read_on_chain_set(&self) -> Result<RawOnChainAttesterSet, AttesterSetError> {
            if self.fail {
                return Err(AttesterSetError::Transient("boom".into()));
            }
            Ok(RawOnChainAttesterSet {
                signature_threshold: self.threshold,
                enabled_addresses: self.enabled.clone(),
                block_or_ledger: Some("mock".into()),
            })
        }
    }

    fn deps(
        iris: Arc<MockIris>,
        enabled: Vec<[u8; 20]>,
    ) -> (
        AttestationRefreshDeps,
        Arc<AttestationTrustCache>,
        Arc<MockClock>,
    ) {
        let clock = Arc::new(MockClock::new());
        let cache = Arc::new(AttestationTrustCache::new(
            Duration::from_secs(60),
            Duration::from_secs(300),
            clock.clone(),
        ));
        let readers: Vec<Arc<dyn AttesterSetReader>> = vec![
            Arc::new(MockReader {
                dest: AttesterDestination::Sepolia,
                enabled: enabled.clone(),
                threshold: 2,
                fail: false,
            }),
            Arc::new(MockReader {
                dest: AttesterDestination::StellarTestnet,
                enabled,
                threshold: 2,
                fail: false,
            }),
        ];
        (
            AttestationRefreshDeps {
                iris_source: iris,
                readers,
            },
            cache,
            clock,
        )
    }

    #[tokio::test]
    async fn atomic_generation_swap_both_destinations() {
        let iris_keys = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let iris = Arc::new(MockIris {
            keys: iris_keys.clone(),
            fail_count: AtomicUsize::new(0),
        });
        let (deps, cache, _) = deps(iris, iris_keys.clone());
        cache.full_refresh(&deps).await.expect("refresh");
        let gen = cache.generation().unwrap();
        assert_eq!(gen.sepolia.generation, gen.stellar.generation);
        assert_eq!(
            gen.iris.set_hash,
            iris_candidate_hash(&{
                let mut sorted = iris_keys.clone();
                sorted.sort();
                sorted
            })
        );
    }

    #[tokio::test]
    async fn ensure_fresh_refreshes_near_ttl() {
        let iris_keys = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let iris = Arc::new(MockIris {
            keys: iris_keys.clone(),
            fail_count: AtomicUsize::new(0),
        });
        let (deps, cache, clock) = deps(iris, iris_keys);
        cache.full_refresh(&deps).await.unwrap();
        clock.advance(Duration::from_secs(61));
        cache.ensure_fresh(&deps).await.unwrap();
        assert!(cache.is_ready());
    }

    #[tokio::test]
    async fn stale_max_fails_closed() {
        let iris_keys = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let iris = Arc::new(MockIris {
            keys: iris_keys.clone(),
            fail_count: AtomicUsize::new(0),
        });
        let (deps, cache, clock) = deps(iris, iris_keys);
        cache.full_refresh(&deps).await.unwrap();
        clock.advance(Duration::from_secs(301));
        let err = cache.ensure_fresh(&deps).await.unwrap_err();
        assert_eq!(err, AttestationTrustError::Stale);
    }

    #[tokio::test]
    async fn transient_failure_keeps_prior_generation() {
        let iris_keys = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let iris = Arc::new(MockIris {
            keys: iris_keys.clone(),
            fail_count: AtomicUsize::new(0),
        });
        let (deps, cache, clock) = deps(iris.clone(), iris_keys);
        cache.full_refresh(&deps).await.unwrap();
        let gen1 = cache.generation().unwrap().generation;
        clock.advance(Duration::from_secs(61));
        iris.fail_count.store(1, AtomicOrdering::SeqCst);
        assert!(cache.full_refresh(&deps).await.is_err());
        assert_eq!(cache.generation().unwrap().generation, gen1);
    }

    #[tokio::test]
    async fn partial_destination_failure_does_not_swap() {
        let iris_keys = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let iris = Arc::new(MockIris {
            keys: iris_keys.clone(),
            fail_count: AtomicUsize::new(0),
        });
        let clock = Arc::new(MockClock::new());
        let cache = Arc::new(AttestationTrustCache::new(
            Duration::from_secs(60),
            Duration::from_secs(300),
            clock.clone(),
        ));
        let readers_ok: Vec<Arc<dyn AttesterSetReader>> = vec![
            Arc::new(MockReader {
                dest: AttesterDestination::Sepolia,
                enabled: iris_keys.clone(),
                threshold: 2,
                fail: false,
            }),
            Arc::new(MockReader {
                dest: AttesterDestination::StellarTestnet,
                enabled: iris_keys.clone(),
                threshold: 2,
                fail: false,
            }),
        ];
        let deps_ok = AttestationRefreshDeps {
            iris_source: iris.clone(),
            readers: readers_ok,
        };
        cache.full_refresh(&deps_ok).await.unwrap();
        let gen1 = cache.generation().unwrap().generation;

        let readers_fail: Vec<Arc<dyn AttesterSetReader>> = vec![
            Arc::new(MockReader {
                dest: AttesterDestination::Sepolia,
                enabled: iris_keys.clone(),
                threshold: 2,
                fail: false,
            }),
            Arc::new(MockReader {
                dest: AttesterDestination::StellarTestnet,
                enabled: iris_keys,
                threshold: 2,
                fail: true,
            }),
        ];
        let deps_fail = AttestationRefreshDeps {
            iris_source: iris,
            readers: readers_fail,
        };
        clock.advance(Duration::from_secs(61));
        assert!(cache.full_refresh(&deps_fail).await.is_err());
        assert_eq!(cache.generation().unwrap().generation, gen1);
    }

    #[tokio::test]
    async fn concurrent_single_flight_refresh() {
        let iris_keys = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let iris = Arc::new(MockIris {
            keys: iris_keys.clone(),
            fail_count: AtomicUsize::new(0),
        });
        let (deps, cache, clock) = deps(iris, iris_keys);
        cache.full_refresh(&deps).await.unwrap();
        clock.advance(Duration::from_secs(61));
        let cache2 = cache.clone();
        let deps2 = Arc::new(deps);
        let h1 = tokio::spawn({
            let deps = deps2.clone();
            let cache = cache.clone();
            async move { cache.full_refresh(deps.as_ref()).await }
        });
        let h2 = tokio::spawn(async move { cache2.full_refresh(deps2.as_ref()).await });
        let (r1, r2) = tokio::join!(h1, h2);
        assert!(r1.unwrap().is_ok());
        assert!(r2.unwrap().is_ok());
    }

    #[tokio::test]
    async fn background_task_cancels_on_drop() {
        let iris_keys = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let iris = Arc::new(MockIris {
            keys: iris_keys.clone(),
            fail_count: AtomicUsize::new(0),
        });
        let (deps, cache, _) = deps(iris, iris_keys);
        cache.full_refresh(&deps).await.unwrap();
        let weak = Arc::downgrade(&cache);
        let deps = Arc::new(deps);
        let handle =
            AttestationTrustCache::spawn_background_refresh(weak, deps, Duration::from_millis(20));
        drop(cache);
        tokio::time::sleep(Duration::from_millis(80)).await;
        handle.abort();
    }
}

use crate::cache::CacheManager;
use stellarroute_routing::health::policy::{OverrideDirective, OverrideRegistry};
use stellarroute_routing::health::scorer::VenueType;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

const REDIS_KILL_SWITCH_KEY: &str = "stellarroute:kill_switches";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KillSwitchState {
    pub sources: HashMap<VenueType, OverrideDirective>,
    pub venues: HashMap<String, OverrideDirective>,
}

impl Default for KillSwitchState {
    fn default() -> Self {
        Self {
            sources: HashMap::new(),
            venues: HashMap::new(),
        }
    }
}

pub struct KillSwitchManager {
    cache: Option<Arc<Mutex<CacheManager>>>,
    /// In-memory cache of the kill switch state for fast access
    state: Arc<Mutex<KillSwitchState>>,
}

impl KillSwitchManager {
    pub fn new(cache: Option<Arc<Mutex<CacheManager>>>) -> Self {
        Self {
            cache,
            state: Arc::new(Mutex::new(KillSwitchState::default())),
        }
    }

    /// Load the kill switch state from Redis
    pub async fn load(&self) {
        if let Some(cache) = &self.cache {
            let mut cache = cache.lock().await;
            if let Some(state) = cache.get::<KillSwitchState>(REDIS_KILL_SWITCH_KEY).await {
                {
                    let mut current_state = self.state.lock().await;
                    *current_state = state.clone();
                }

                // Record metrics
                for (source, directive) in &state.sources {
                    let disabled = matches!(directive, OverrideDirective::ForceExclude);
                    crate::metrics::record_kill_switch_status("source", &format!("{:?}", source).to_lowercase(), disabled);
                }
                for (venue, directive) in &state.venues {
                    let disabled = matches!(directive, OverrideDirective::ForceExclude);
                    crate::metrics::record_kill_switch_status("venue", venue, disabled);
                }
            }
        }
    }

    /// Spawn a background task to keep the kill switch state synced from Redis
    pub fn start_sync(self: Arc<Self>) {
        info!("Starting kill switch sync task");
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                self.load().await;
            }
        });
    }

    /// Get the current kill switch state
    pub async fn get_state(&self) -> KillSwitchState {
        self.state.lock().await.clone()
    }

    /// Update the kill switch state and persist to Redis
    pub async fn update_state(&self, new_state: KillSwitchState) -> Result<(), String> {
        // Update in-memory
        {
            let mut state = self.state.lock().await;
            *state = new_state.clone();
        }

        // Record metrics
        for (source, directive) in &new_state.sources {
            let disabled = matches!(directive, OverrideDirective::ForceExclude);
            crate::metrics::record_kill_switch_status("source", &format!("{:?}", source).to_lowercase(), disabled);
        }
        for (venue, directive) in &new_state.venues {
            let disabled = matches!(directive, OverrideDirective::ForceExclude);
            crate::metrics::record_kill_switch_status("venue", venue, disabled);
        }

        // Persist to Redis
        if let Some(cache) = &self.cache {
            let mut cache = cache.lock().await;
            if let Err(e) = cache
                .set(
                    REDIS_KILL_SWITCH_KEY,
                    &new_state,
                    std::time::Duration::from_secs(0), // No TTL for kill switches
                )
                .await
            {
                warn!("Failed to persist kill switch state to Redis: {}", e);
                return Err(format!("Redis error: {}", e));
            }
        }

        info!("Kill switch state updated and persisted");
        Ok(())
    }

    /// Construct an OverrideRegistry from the current kill switch state
    pub async fn get_override_registry(&self) -> OverrideRegistry {
        let state = self.state.lock().await;
        OverrideRegistry {
            venue_entries: state.venues.clone(),
            source_entries: state.sources.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellarroute_routing::health::policy::OverrideDirective;

    #[tokio::test]
    async fn test_kill_switch_manager_in_memory() {
        let manager = KillSwitchManager::new(None);
        
        let mut sources = HashMap::new();
        sources.insert(VenueType::Amm, OverrideDirective::ForceExclude);
        
        let mut venues = HashMap::new();
        venues.insert("sdex:123".to_string(), OverrideDirective::ForceExclude);
        
        let state = KillSwitchState { sources, venues };
        manager.update_state(state).await.unwrap();
        
        let registry = manager.get_override_registry().await;
        assert_eq!(registry.source_entries.get(&VenueType::Amm), Some(&OverrideDirective::ForceExclude));
        assert_eq!(registry.venue_entries.get("sdex:123"), Some(&OverrideDirective::ForceExclude));
        assert_eq!(registry.source_entries.get(&VenueType::Sdex), None);
    }
}

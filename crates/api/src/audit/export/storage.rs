//! Object storage backend for audit log export batches.

use chrono::Datelike;
use std::path::{Path, PathBuf};

use crate::error::{ApiError, Result};

/// Writes exported audit batches to a filesystem directory (sync to S3/GCS via ops tooling).
pub struct LocalObjectStorage {
    root: PathBuf,
}

impl LocalObjectStorage {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Write `payload` to `object_key` relative to the storage root.
    pub async fn put_object(&self, object_key: &str, payload: &[u8]) -> Result<()> {
        let path = self.root.join(object_key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
                    "Failed to create export directory {}: {}",
                    parent.display(),
                    e
                )))
            })?;
        }

        tokio::fs::write(&path, payload).await.map_err(|e| {
            ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
                "Failed to write export object {}: {}",
                path.display(),
                e
            )))
        })?;

        Ok(())
    }
}

/// Build an object key suitable for incident replay workflows.
///
/// Layout: `{prefix}/{YYYY}/{MM}/{DD}/batch-{first_id}-{last_id}-{unix_ts}.jsonl.gz`
pub fn build_object_key(prefix: &str, first_id: i64, last_id: i64, unix_ts: i64) -> String {
    let now = chrono::DateTime::<chrono::Utc>::from_timestamp(unix_ts, 0)
        .unwrap_or_else(chrono::Utc::now);
    format!(
        "{}/{:04}/{:02}/{:02}/batch-{}-{}-{}.jsonl.gz",
        prefix.trim_matches('/'),
        now.year(),
        now.month(),
        now.day(),
        first_id,
        last_id,
        unix_ts
    )
}

/// Build a manifest key alongside the data object for replay tooling.
pub fn build_manifest_key(data_object_key: &str) -> String {
    format!(
        "{}.manifest.json",
        data_object_key.trim_end_matches(".jsonl.gz")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Utc};

    #[test]
    fn object_key_includes_date_partition_and_id_range() {
        let ts = 1_700_000_000_i64;
        let key = build_object_key("stellarroute/audit-logs", 100, 250, ts);
        let now = Utc.timestamp_opt(ts, 0).single().unwrap();
        assert!(key.contains(&format!("{:04}", now.year())));
        assert!(key.contains("batch-100-250-"));
        assert!(key.ends_with(".jsonl.gz"));
    }

    #[tokio::test]
    async fn local_storage_writes_file() {
        let dir = std::env::temp_dir().join(format!(
            "stellarroute-audit-export-{}",
            uuid::Uuid::new_v4()
        ));
        let storage = LocalObjectStorage::new(&dir);
        let key = "stellarroute/audit-logs/2026/06/24/batch-1-2-1.jsonl.gz";
        storage.put_object(key, b"test").await.unwrap();
        assert!(dir.join(key).exists());
        let _ = tokio::fs::remove_dir_all(dir).await;
    }
}

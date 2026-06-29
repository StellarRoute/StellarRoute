//! Contract version metadata endpoint
//!
//! Exposes deployed contract version/hash via API for clients and SDKs.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{error::Result, state::AppState};

/// Contract version metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContractVersionMetadata {
    /// Contract name/identifier
    pub contract_name: String,
    /// Semantic version (e.g., "1.2.3")
    pub version: String,
    /// WASM hash (hex-encoded)
    pub wasm_hash: String,
    /// Network identifier (e.g., "mainnet", "testnet", "futurenet")
    pub network: String,
    /// Contract address on the network
    pub contract_address: Option<String>,
    /// Deployment timestamp (Unix timestamp in seconds)
    pub deployed_at: Option<i64>,
    /// Git commit SHA that built this version
    pub git_commit: Option<String>,
}

/// List all registered contract versions
#[utoipa::path(
    get,
    path = "/api/v1/contracts/registry",
    responses(
        (status = 200, description = "List of contract versions", body = Vec<ContractVersionMetadata>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Contracts"
)]
pub async fn list_contract_versions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ContractVersionMetadata>>> {
    // Query from database or config
    let rows = sqlx::query(
        r#"
        SELECT 
            contract_name,
            version,
            wasm_hash,
            network,
            contract_address,
            deployed_at,
            git_commit
        FROM contract_registry
        ORDER BY deployed_at DESC
        "#,
    )
    .fetch_all(state.db.read_pool())
    .await?;

    let contracts = rows.into_iter().map(contract_from_row).collect();
    Ok(Json(contracts))
}

/// Get specific contract version metadata
#[utoipa::path(
    get,
    path = "/api/v1/contracts/registry/{contract_name}",
    params(
        ("contract_name" = String, Path, description = "Contract name identifier")
    ),
    responses(
        (status = 200, description = "Contract version metadata", body = ContractVersionMetadata),
        (status = 404, description = "Contract not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Contracts"
)]
pub async fn get_contract_version(
    State(state): State<Arc<AppState>>,
    Path(contract_name): Path<String>,
) -> Result<Json<ContractVersionMetadata>> {
    let row = sqlx::query(
        r#"
        SELECT 
            contract_name,
            version,
            wasm_hash,
            network,
            contract_address,
            deployed_at,
            git_commit
        FROM contract_registry
        WHERE contract_name = $1
        ORDER BY deployed_at DESC
        LIMIT 1
        "#,
    )
    .bind(&contract_name)
    .fetch_optional(state.db.read_pool())
    .await?
    .ok_or_else(|| {
        crate::error::ApiError::NotFound(format!("Contract '{}' not found", contract_name))
    })?;

    Ok(Json(contract_from_row(row)))
}

/// Get contract version by network
#[utoipa::path(
    get,
    path = "/api/v1/contracts/registry/{contract_name}/network/{network}",
    params(
        ("contract_name" = String, Path, description = "Contract name identifier"),
        ("network" = String, Path, description = "Network identifier (mainnet, testnet, futurenet)")
    ),
    responses(
        (status = 200, description = "Contract version metadata", body = ContractVersionMetadata),
        (status = 404, description = "Contract not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Contracts"
)]
pub async fn get_contract_version_by_network(
    State(state): State<Arc<AppState>>,
    Path((contract_name, network)): Path<(String, String)>,
) -> Result<Json<ContractVersionMetadata>> {
    let row = sqlx::query(
        r#"
        SELECT 
            contract_name,
            version,
            wasm_hash,
            network,
            contract_address,
            deployed_at,
            git_commit
        FROM contract_registry
        WHERE contract_name = $1 AND network = $2
        ORDER BY deployed_at DESC
        LIMIT 1
        "#,
    )
    .bind(&contract_name)
    .bind(&network)
    .fetch_optional(state.db.read_pool())
    .await?
    .ok_or_else(|| {
        crate::error::ApiError::NotFound(format!(
            "Contract '{}' not found on network '{}'",
            contract_name, network
        ))
    })?;

    Ok(Json(contract_from_row(row)))
}

fn contract_from_row(row: sqlx::postgres::PgRow) -> ContractVersionMetadata {
    ContractVersionMetadata {
        contract_name: row.get("contract_name"),
        version: row.get("version"),
        wasm_hash: row.get("wasm_hash"),
        network: row.get("network"),
        contract_address: row.get("contract_address"),
        deployed_at: row.get("deployed_at"),
        git_commit: row.get("git_commit"),
    }
}

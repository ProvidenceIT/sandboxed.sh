//! Workspace management API endpoints.
//!
//! Provides endpoints for managing execution workspaces:
//! - List workspaces
//! - Create workspace
//! - Get workspace details
//! - Delete workspace

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::workspace::{Workspace, WorkspaceStatus, WorkspaceType};

/// Create workspace routes.
pub fn routes() -> Router<Arc<super::routes::AppState>> {
    Router::new()
        .route("/", get(list_workspaces))
        .route("/", post(create_workspace))
        .route("/:id", get(get_workspace))
        .route("/:id", delete(delete_workspace))
}

// ─────────────────────────────────────────────────────────────────────────────
// Request/Response Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    /// Human-readable name
    pub name: String,
    /// Type of workspace (defaults to "host")
    #[serde(default)]
    pub workspace_type: WorkspaceType,
    /// Working directory path (optional, defaults based on type)
    pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceResponse {
    pub id: Uuid,
    pub name: String,
    pub workspace_type: WorkspaceType,
    pub path: PathBuf,
    pub status: WorkspaceStatus,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<Workspace> for WorkspaceResponse {
    fn from(w: Workspace) -> Self {
        Self {
            id: w.id,
            name: w.name,
            workspace_type: w.workspace_type,
            path: w.path,
            status: w.status,
            error_message: w.error_message,
            created_at: w.created_at,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/workspaces - List all workspaces.
async fn list_workspaces(
    State(state): State<Arc<super::routes::AppState>>,
) -> Result<Json<Vec<WorkspaceResponse>>, (StatusCode, String)> {
    let workspaces = state.workspaces.list().await;
    let responses: Vec<WorkspaceResponse> = workspaces.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

/// Validate workspace name to prevent path traversal.
fn validate_workspace_name(name: &str) -> Result<(), (StatusCode, String)> {
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name cannot be empty".to_string()));
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err((
            StatusCode::BAD_REQUEST,
            "Name contains invalid characters".to_string(),
        ));
    }
    if name.starts_with('.') {
        return Err((
            StatusCode::BAD_REQUEST,
            "Name cannot start with a dot".to_string(),
        ));
    }
    Ok(())
}

/// POST /api/workspaces - Create a new workspace.
async fn create_workspace(
    State(state): State<Arc<super::routes::AppState>>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<Json<WorkspaceResponse>, (StatusCode, String)> {
    // Validate workspace name for path traversal
    validate_workspace_name(&req.name)?;

    // Determine path
    let path = match &req.path {
        Some(custom_path) => {
            // For custom paths, validate they're within the working directory
            let working_dir = &state.config.working_dir;
            let canonical_working = working_dir.canonicalize().unwrap_or_else(|_| working_dir.clone());
            let canonical_custom = custom_path.canonicalize().unwrap_or_else(|_| custom_path.clone());

            if !canonical_custom.starts_with(&canonical_working) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Custom path must be within the working directory".to_string(),
                ));
            }
            custom_path.clone()
        }
        None => match req.workspace_type {
            WorkspaceType::Host => state.config.working_dir.clone(),
            WorkspaceType::Chroot => {
                // Chroot workspaces go in a dedicated directory
                state
                    .config
                    .working_dir
                    .join(".openagent/chroots")
                    .join(&req.name)
            }
        },
    };

    let workspace = match req.workspace_type {
        WorkspaceType::Host => Workspace {
            id: Uuid::new_v4(),
            name: req.name,
            workspace_type: WorkspaceType::Host,
            path,
            status: WorkspaceStatus::Ready,
            error_message: None,
            config: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        },
        WorkspaceType::Chroot => Workspace::new_chroot(req.name, path),
    };

    let id = state.workspaces.add(workspace.clone()).await;
    let response: WorkspaceResponse = workspace.into();

    tracing::info!("Created workspace: {} ({})", response.name, id);

    Ok(Json(response))
}

/// GET /api/workspaces/:id - Get workspace details.
async fn get_workspace(
    State(state): State<Arc<super::routes::AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkspaceResponse>, (StatusCode, String)> {
    state
        .workspaces
        .get(id)
        .await
        .map(|w| Json(w.into()))
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Workspace {} not found", id)))
}

/// DELETE /api/workspaces/:id - Delete a workspace.
async fn delete_workspace(
    State(state): State<Arc<super::routes::AppState>>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    if id == crate::workspace::DEFAULT_WORKSPACE_ID {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot delete default host workspace".to_string(),
        ));
    }

    if state.workspaces.delete(id).await {
        Ok((
            StatusCode::OK,
            format!("Workspace {} deleted successfully", id),
        ))
    } else {
        Err((StatusCode::NOT_FOUND, format!("Workspace {} not found", id)))
    }
}

//! 数据备份 API

use axum::{
    extract::Json,
    Router,
    routing::{get, post},
};
use serde::Serialize;

/// 备份路由
pub fn router() -> Router {
    Router::new()
        .route("/", get(list_backups))
        .route("/upload", post(upload_backup))
        .route("/download/:id", get(download_backup))
}

/// 备份信息
#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub id: String,
    pub created_at: String,
    pub size: u64,
}

/// 备份列表
async fn list_backups() -> Json<Vec<BackupInfo>> {
    // TODO: 实现备份列表
    Json(vec![])
}

/// 上传备份
async fn upload_backup() -> Json<()> {
    // TODO: 实现上传逻辑
    Json(())
}

/// 下载备份
async fn download_backup() -> Json<()> {
    // TODO: 实现下载逻辑
    Json(())
}
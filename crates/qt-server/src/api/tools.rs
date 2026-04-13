//! 工具分发 API

use axum::{
    extract::Json,
    Router,
    routing::{get, post},
};
use serde::Serialize;

/// 工具路由
pub fn router() -> Router {
    Router::new()
        .route("/", get(list_tools))
        .route("/:id", get(get_tool))
        .route("/:id/download", get(download_tool))
        .route("/:id/rate", post(rate_tool))
}

/// 工具信息
#[derive(Debug, Serialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub download_count: u64,
    pub rating: f32,
}

/// 工具列表
async fn list_tools() -> Json<Vec<ToolInfo>> {
    // TODO: 实现工具列表
    Json(vec![])
}

/// 获取工具详情
async fn get_tool() -> Json<ToolInfo> {
    // TODO: 实现获取详情
    Json(ToolInfo {
        id: "dummy".to_string(),
        name: "Dummy Tool".to_string(),
        description: "A dummy tool".to_string(),
        version: "1.0.0".to_string(),
        author: "Unknown".to_string(),
        download_count: 0,
        rating: 0.0,
    })
}

/// 下载工具
async fn download_tool() -> Json<()> {
    // TODO: 实现下载逻辑
    Json(())
}

/// 工具评分
async fn rate_tool() -> Json<()> {
    // TODO: 实现评分逻辑
    Json(())
}
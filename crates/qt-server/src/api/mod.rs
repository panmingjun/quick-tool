//! REST API 模块

pub mod auth;
pub mod backup;
pub mod tools;

use axum::Router;

/// 创建 API 路由
pub fn create_router() -> Router {
    Router::new()
        .nest("/api/v1/auth", auth::router())
        .nest("/api/v1/backup", backup::router())
        .nest("/api/v1/tools", tools::router())
}
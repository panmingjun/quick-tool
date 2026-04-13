//! Quick Tool 服务端
//!
//! 提供账号管理、数据备份和工具分发服务

mod api;
mod db;
mod service;

use axum::Router;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 服务端配置
pub struct ServerConfig {
    /// 服务端口
    pub port: u16,
    /// 数据库连接字符串
    pub database_url: String,
    /// JWT 密钥
    pub jwt_secret: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/quicktool".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev_secret_change_in_production".to_string()),
        }
    }
}

#[tokio::main]
async fn main() {
    // 初始化日志（默认 info 级别）
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = ServerConfig::default();

    tracing::info!("Quick Tool 服务端启动");
    tracing::info!("端口: {}", config.port);
    tracing::info!("数据库: {}", config.database_url);

    // 创建 API 路由
    let app = Router::new()
        .nest("/api/v1/auth", api::auth::router())
        .nest("/api/v1/backup", api::backup::router())
        .nest("/api/v1/tools", api::tools::router());

    // 启动 HTTP 服务
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("服务监听地址: {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("无法绑定端口");

    tracing::info!("服务已启动，等待连接...");

    axum::serve(listener, app)
        .await
        .expect("服务启动失败");
}
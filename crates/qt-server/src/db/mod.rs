//! PostgreSQL 数据库模块

pub mod schema;

/// 数据库连接池类型
pub type DbPool = sqlx::PgPool;

/// 初始化数据库连接池
pub async fn init_pool(database_url: &str) -> qt_core::Result<DbPool> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .map_err(|e| qt_core::Error::Database(format!("数据库连接失败: {}", e)))
}
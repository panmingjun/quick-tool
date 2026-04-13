//! 数据库 Schema 定义

/// 用户表 Schema
pub const USERS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
";

/// 备份表 Schema
pub const BACKUPS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS backups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    data_type TEXT NOT NULL,
    encrypted_data BYTEA NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
";

/// 工具表 Schema
pub const TOOLS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    description TEXT,
    version TEXT NOT NULL,
    author TEXT NOT NULL,
    wasm_data BYTEA NOT NULL,
    download_count INTEGER DEFAULT 0,
    rating REAL DEFAULT 0,
    created_at TIMESTAMP DEFAULT NOW()
);
";

/// 创建所有表
pub async fn create_tables(pool: &sqlx::PgPool) -> qt_core::Result<()> {
    sqlx::query(USERS_SCHEMA)
        .execute(pool)
        .await
        .map_err(|e| qt_core::Error::Database(format!("创建用户表失败: {}", e)))?;

    sqlx::query(BACKUPS_SCHEMA)
        .execute(pool)
        .await
        .map_err(|e| qt_core::Error::Database(format!("创建备份表失败: {}", e)))?;

    sqlx::query(TOOLS_SCHEMA)
        .execute(pool)
        .await
        .map_err(|e| qt_core::Error::Database(format!("创建工具表失败: {}", e)))?;

    Ok(())
}
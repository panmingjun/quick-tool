//! 数据库连接与初始化
//!
//! 提供 SQLite 数据库的连接管理。
//! 每个插件源对应一个独立的数据库文件，路径为 `<data_dir>/<source_id>.sqlite`。

use std::path::PathBuf;
use sqlx::Row;

/// 数据库管理器
pub struct DatabaseManager {
    /// 数据根目录
    data_root: PathBuf,
}

impl DatabaseManager {
    /// 创建新的数据库管理器
    pub fn new(data_root: PathBuf) -> Self {
        Self { data_root }
    }

    /// 获取插件源对应的数据库路径
    pub fn source_db_path(&self, source_id: &str) -> PathBuf {
        self.data_root.join(format!("{source_id}.sqlite"))
    }

    /// 获取插件源对应的连接字符串
    pub fn source_db_uri(&self, source_id: &str) -> String {
        let path = self.source_db_path(source_id);
        format!("sqlite:{}", path.display())
    }

    /// 确保数据目录存在
    fn ensure_dir(&self) -> anyhow::Result<()> {
        if !self.data_root.exists() {
            std::fs::create_dir_all(&self.data_root)
                .map_err(|e| anyhow::anyhow!("创建数据目录失败: {e}"))?;
        }
        Ok(())
    }

    /// 初始化插件源数据库（创建表）
    ///
    /// 当前为预留接口，具体表的创建由各插件模块（如 markdown）调用。
    pub async fn init_source_db(&self, source_id: &str) -> anyhow::Result<sqlx::SqlitePool> {
        self.ensure_dir()?;

        let uri = self.source_db_uri(source_id);
        let pool = sqlx::SqlitePool::connect(&uri)
            .await
            .map_err(|e| anyhow::anyhow!("连接数据库失败: {e}"))?;

        // 创建通用元数据表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("创建元数据表失败: {e}"))?;

        Ok(pool)
    }

    /// 获取或初始化插件源数据库
    pub async fn get_or_init_source_db(
        &self,
        source_id: &str,
    ) -> anyhow::Result<sqlx::SqlitePool> {
        let uri = self.source_db_uri(source_id);

        // 先尝试连接，如果表不存在则初始化
        match sqlx::SqlitePool::connect(&uri).await {
            Ok(pool) => {
                // 检查 _metadata 表是否存在
                let exists = sqlx::query(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='_metadata'",
                )
                .fetch_one(&pool)
                .await
                .ok()
                .and_then(|row| row.get::<Option<i64>, _>(0))
                .unwrap_or(0)
                    > 0;

                if !exists {
                    // 重新连接以创建表
                    drop(pool);
                    self.init_source_db(source_id).await
                } else {
                    Ok(pool)
                }
            }
            Err(_) => self.init_source_db(source_id).await,
        }
    }
}

//! 插件独立键值存储
//!
//! 每个插件一个独立 SQLite 库文件（`<data_root>/<plugin_id>/data.sqlite`），
//! 数据按插件完全隔离。表结构：
//! - `kv`：键值对
//!   - `key` TEXT PRIMARY KEY
//!   - `value` TEXT NOT NULL
//!   - `updated_at` TEXT
//!
//! 配额按 key 与 value 的 UTF-8 字节长度累计，写入前检查，超限拒绝。
//! 采用同步 rusqlite（客户端无 tokio runtime，宿主在插件调用链路内同步执行）。

use std::path::Path;

use rusqlite::Connection;

/// 默认配额：10 MiB / 插件
pub const DEFAULT_QUOTA_BYTES: u64 = 10 * 1024 * 1024;

/// 键值存储错误（与 WIT `storage.kv-error` 对应）
#[derive(Debug)]
pub enum KvError {
    /// 超出配额，携带配额上限（字节）
    QuotaExceeded(u64),
    /// 底层数据库/IO 错误
    Io(String),
}

impl KvError {
    /// 底层错误的便捷构造
    fn io(e: impl std::fmt::Display) -> Self {
        Self::Io(e.to_string())
    }
}

/// 校验插件 ID 可安全用作目录名（防路径穿越）
fn validate_plugin_id(plugin_id: &str) -> Result<(), KvError> {
    let valid = !plugin_id.is_empty()
        && plugin_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !plugin_id.starts_with('.');
    if valid {
        Ok(())
    } else {
        Err(KvError::Io(format!("非法插件 ID: {plugin_id}")))
    }
}

/// 插件独立键值存储
pub struct PluginStore {
    conn: Connection,
    quota_bytes: u64,
}

impl PluginStore {
    /// 打开（或初始化）插件的独立存储库
    ///
    /// 库文件位于 `<data_root>/<plugin_id>/data.sqlite`，首次打开自动建表。
    pub fn open(data_root: &Path, plugin_id: &str, quota_bytes: u64) -> Result<Self, KvError> {
        validate_plugin_id(plugin_id)?;

        let dir = data_root.join(plugin_id);
        std::fs::create_dir_all(&dir).map_err(KvError::io)?;

        let conn = Connection::open(dir.join("data.sqlite")).map_err(KvError::io)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .map_err(KvError::io)?;

        Ok(Self { conn, quota_bytes })
    }

    /// 当前已用字节数（key 与 value 的 UTF-8 字节长度之和）
    ///
    /// 注：SQLite `LENGTH(TEXT)` 返回字符数，须 `CAST AS BLOB` 后取字节数。
    pub fn used_bytes(&self) -> Result<u64, KvError> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(CAST(key AS BLOB)) + LENGTH(CAST(value AS BLOB))), 0)
                 FROM kv",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n.max(0) as u64)
            .map_err(KvError::io)
    }

    /// 读取指定键的值；不存在返回 None
    pub fn kv_get(&self, key: &str) -> Result<Option<String>, KvError> {
        let mut stmt = self.conn.prepare("SELECT value FROM kv WHERE key = ?1").map_err(KvError::io)?;
        let mut rows = stmt.query([key]).map_err(KvError::io)?;
        match rows.next().map_err(KvError::io)? {
            Some(row) => Ok(Some(row.get(0).map_err(KvError::io)?)),
            None => Ok(None),
        }
    }

    /// 写入键值（已存在则覆盖）；超出配额时拒绝并返回错误
    pub fn kv_set(&mut self, key: &str, value: &str) -> Result<(), KvError> {
        // 增量 = 新增 (key+value) 字节 − 被覆盖的旧 value 字节
        let old_value_bytes: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(LENGTH(CAST(value AS BLOB)), 0) FROM kv WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let new_total = self.used_bytes()? as i64 + (key.len() + value.len()) as i64
            - old_value_bytes;
        if new_total > self.quota_bytes as i64 {
            return Err(KvError::QuotaExceeded(self.quota_bytes));
        }

        self.conn
            .execute(
                "INSERT INTO kv (key, value, updated_at)
                 VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                rusqlite::params![key, value],
            )
            .map_err(KvError::io)?;
        Ok(())
    }

    /// 删除指定键；键不存在视为成功
    pub fn kv_delete(&mut self, key: &str) -> Result<(), KvError> {
        self.conn
            .execute("DELETE FROM kv WHERE key = ?1", [key])
            .map_err(KvError::io)?;
        Ok(())
    }

    /// 列出全部键（字典序）
    pub fn kv_keys(&self) -> Result<Vec<String>, KvError> {
        let mut stmt = self.conn.prepare("SELECT key FROM kv ORDER BY key").map_err(KvError::io)?;
        let keys = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(KvError::io)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(KvError::io)?;
        Ok(keys)
    }
}

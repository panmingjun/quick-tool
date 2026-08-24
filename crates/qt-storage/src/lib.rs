//! 本地数据存储模块
//!
//! 使用 SQLite 作为持久化存储引擎。
//! - 按插件源分库：每个插件源对应一个独立的 `.sqlite` 文件
//! - 插件独立键值存储：每个插件一个独立库（`per_plugin`），KV 语义 + 配额限制
//! - Markdown 记事本数据使用结构化表存储（文件夹、笔记）

pub mod db;
pub mod markdown;
pub mod per_plugin;

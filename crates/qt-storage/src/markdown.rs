//! Markdown 记事本数据存储
//!
//! 按插件源分库、按插件分表存储 Markdown 笔记数据。
//!
//! 表结构：
//! - `folders`：文件夹（目录层级）
//!   - `id` TEXT PRIMARY KEY
//!   - `name` TEXT NOT NULL
//!   - `parent_id` TEXT （父文件夹 ID，NULL 表示根）
//!   - `sort_order` INTEGER DEFAULT 0
//!   - `created_at` TEXT
//!   - `updated_at` TEXT
//! - `notes`：笔记
//!   - `id` TEXT PRIMARY KEY
//!   - `folder_id` TEXT （所属文件夹）
//!   - `title` TEXT NOT NULL
//!   - `content` TEXT （Markdown 内容）
//!   - `sort_order` INTEGER DEFAULT 0
//!   - `created_at` TEXT
//!   - `updated_at` TEXT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// 文件夹记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Folder {
    /// 文件夹 ID
    pub id: String,
    /// 文件夹名称
    pub name: String,
    /// 父文件夹 ID
    pub parent_id: Option<String>,
    /// 排序顺序
    pub sort_order: i64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 笔记记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Note {
    /// 笔记 ID
    pub id: String,
    /// 所属文件夹 ID
    pub folder_id: String,
    /// 笔记标题
    pub title: String,
    /// Markdown 内容
    pub content: String,
    /// 排序顺序
    pub sort_order: i64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 文件夹树节点（递归结构，用于左侧目录树展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderNode {
    /// 文件夹信息
    pub folder: Folder,
    /// 子文件夹
    pub children: Vec<FolderNode>,
    /// 该文件夹下的笔记列表
    pub notes: Vec<Note>,
}

/// Markdown 记事本数据库操作
pub struct MarkdownDb {
    /// 数据库连接池
    pool: SqlitePool,
}

impl MarkdownDb {
    /// 创建新的 Markdown 数据库操作器
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 初始化表结构
    pub async fn init(&self) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS folders (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                parent_id TEXT,
                sort_order INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("创建 folders 表失败: {e}"))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                folder_id TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                sort_order INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (folder_id) REFERENCES folders(id)
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("创建 notes 表失败: {e}"))?;

        Ok(())
    }

    /// 获取文件夹树（完整层级结构）
    pub async fn get_folder_tree(&self) -> anyhow::Result<Vec<FolderNode>> {
        let folders: Vec<Folder> = sqlx::query_as::<_, Folder>(
            "SELECT * FROM folders ORDER BY sort_order ASC, created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("查询文件夹列表失败: {e}"))?;

        let notes: Vec<Note> = sqlx::query_as::<_, Note>(
            "SELECT * FROM notes ORDER BY sort_order ASC, created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("查询笔记列表失败: {e}"))?;

        // 构建文件夹树
        let mut root_folders: Vec<FolderNode> = Vec::new();
        let mut folder_map: std::collections::HashMap<String, FolderNode> =
            std::collections::HashMap::new();

        // 先创建所有文件夹节点
        for folder in &folders {
            let node = FolderNode {
                folder: folder.clone(),
                children: Vec::new(),
                notes: Vec::new(),
            };
            folder_map.insert(folder.id.clone(), node);
        }

        // 构建树形结构
        for folder in &folders {
            let Some(node) = folder_map.remove(&folder.id) else {
                continue;
            };
            if let Some(parent_id) = &folder.parent_id {
                if let Some(parent) = folder_map.get_mut(parent_id) {
                    parent.children.push(node);
                } else {
                    root_folders.push(node);
                }
            } else {
                root_folders.push(node);
            }
        }

        // 将笔记分配到对应文件夹
        for note in notes {
            if let Some(node) = folder_map.get_mut(&note.folder_id) {
                node.notes.push(note);
            } else if let Some(root) = root_folders
                .iter_mut()
                .find(|r| r.folder.id == note.folder_id)
            {
                root.notes.push(note);
            }
        }

        Ok(root_folders)
    }

    /// 创建文件夹
    pub async fn create_folder(
        &self,
        name: &str,
        parent_id: Option<&str>,
    ) -> anyhow::Result<Folder> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO folders (id, name, parent_id, sort_order, created_at, updated_at)
             VALUES (?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(parent_id)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("创建文件夹失败: {e}"))?;

        Ok(Folder {
            id,
            name: name.to_string(),
            parent_id: parent_id.map(String::from),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// 删除文件夹（级联删除子文件夹和笔记）
    pub async fn delete_folder(&self, folder_id: &str) -> anyhow::Result<()> {
        sqlx::query(
            "DELETE FROM notes WHERE folder_id IN (
                SELECT id FROM folders WHERE id = ? OR parent_id = ?
            )",
        )
        .bind(folder_id)
        .bind(folder_id)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("删除文件夹笔记失败: {e}"))?;

        sqlx::query("DELETE FROM folders WHERE id = ?")
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("删除文件夹失败: {e}"))?;

        Ok(())
    }

    /// 创建笔记
    pub async fn create_note(
        &self,
        folder_id: &str,
        title: &str,
        content: &str,
    ) -> anyhow::Result<Note> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO notes (id, folder_id, title, content, sort_order, created_at, updated_at)
             VALUES (?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(folder_id)
        .bind(title)
        .bind(content)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("创建笔记失败: {e}"))?;

        Ok(Note {
            id,
            folder_id: folder_id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// 更新笔记内容
    pub async fn update_note(
        &self,
        note_id: &str,
        content: &str,
        title: Option<&str>,
    ) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();

        if let Some(t) = title {
            sqlx::query(
                "UPDATE notes SET title = ?, content = ?, updated_at = ? WHERE id = ?",
            )
            .bind(t)
            .bind(content)
            .bind(&now)
            .bind(note_id)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("更新笔记失败: {e}"))?;
        } else {
            sqlx::query(
                "UPDATE notes SET content = ?, updated_at = ? WHERE id = ?",
            )
            .bind(content)
            .bind(&now)
            .bind(note_id)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("更新笔记失败: {e}"))?;
        }

        Ok(())
    }

    /// 删除笔记
    pub async fn delete_note(&self, note_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(note_id)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("删除笔记失败: {e}"))?;
        Ok(())
    }

    /// 获取单个笔记
    pub async fn get_note(&self, note_id: &str) -> anyhow::Result<Option<Note>> {
        sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE id = ?")
            .bind(note_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("查询笔记失败: {e}"))
    }

    /// 获取文件夹下的所有笔记
    pub async fn get_notes_by_folder(&self, folder_id: &str) -> anyhow::Result<Vec<Note>> {
        sqlx::query_as::<_, Note>(
            "SELECT * FROM notes WHERE folder_id = ? ORDER BY sort_order ASC, created_at ASC",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("查询笔记列表失败: {e}"))
    }
}

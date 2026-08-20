//! 公共类型定义
//!
//! 包含工具元信息、插件源、插件列表等核心数据结构

use crate::PluginFeature;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 工具唯一标识
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ToolId(pub Uuid);

impl ToolId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ToolId {
    fn default() -> Self {
        Self::new()
    }
}

/// 服务端唯一标识
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ServerId(pub Uuid);

impl ServerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ServerId {
    fn default() -> Self {
        Self::new()
    }
}

/// 用户唯一标识
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// 工具元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub id: ToolId,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub keywords: Vec<String>,
    pub capabilities: Vec<Capability>,
}

/// 工具能力
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Capability {
    Storage,
    Network,
    Clipboard,
    Screen,
    SystemInfo,
}

/// 服务端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: ServerId,
    pub name: String,
    pub address: String,
    pub is_default: bool,
}

/// 服务端连接状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Error(String),
}

/// 服务端连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnection {
    /// 服务端配置
    pub config: ServerConfig,
    /// 认证令牌
    pub auth_token: Option<String>,
    /// 连接状态
    pub status: ConnectionStatus,
}

/// 插件源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginSourceKind {
    /// 本地文件系统源（指向本地目录）
    Local,
    /// 远程 HTTP/HTTPS 源（指向远程 JSON 文件）
    Remote,
}

impl Default for PluginSourceKind {
    fn default() -> Self {
        Self::Local
    }
}

/// 插件源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSource {
    /// 插件源唯一标识
    pub id: String,
    /// 插件源名称
    pub name: String,
    /// 插件源类型
    pub kind: PluginSourceKind,
    /// 源地址（本地路径或远程 URL）
    pub url: String,
    /// 是否启用
    pub enabled: bool,
    /// 最后更新时间
    #[serde(default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 插件源配置（对应 config.json 中的 plugin_sources 数组）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginSourceConfig {
    /// 插件源列表
    pub sources: Vec<PluginSource>,
}

/// 插件列表（对应插件源 JSON 文件的内容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginList {
    /// 插件源 ID（与 PluginSource.id 对应）
    pub source_id: String,
    /// 插件清单列表
    pub plugins: Vec<PluginEntry>,
}

/// 单个插件条目（从插件源 JSON 中描述）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// 插件 ID
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 版本号
    #[serde(default)]
    pub version: String,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// WASM 下载地址（远程源）或本地路径（本地源）
    pub wasm_url: String,
    /// 功能指令
    #[serde(default)]
    pub features: Vec<PluginFeature>,
}

/// 完整应用配置（config.json 结构）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// 插件源配置
    #[serde(default)]
    pub plugin_sources: PluginSourceConfig,
    /// 插件本地安装根目录
    #[serde(default)]
    pub plugin_install_dir: Option<String>,
    /// 数据目录（SQLite 等）
    #[serde(default)]
    pub data_dir: Option<String>,
    /// 最近使用的插件 ID 列表
    #[serde(default)]
    pub recent_plugins: Vec<String>,
}

impl AppConfig {
    /// 从文件路径加载配置
    pub fn from_file(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::Error::Config(format!("读取配置文件失败: {e}")))?;
        let config: Self = serde_json::from_str(&content)
            .map_err(|e| crate::Error::Config(format!("解析配置文件失败: {e}")))?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn to_file(&self, path: &std::path::Path) -> crate::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::Error::Config(format!("创建配置目录失败: {e}")))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::Config(format!("序列化配置失败: {e}")))?;
        std::fs::write(path, content)
            .map_err(|e| crate::Error::Config(format!("写入配置文件失败: {e}")))?;
        Ok(())
    }
}

/// 默认的本地插件源 ID
pub const DEFAULT_LOCAL_SOURCE_ID: &str = "local";
/// 默认本地插件源名称
pub const DEFAULT_LOCAL_SOURCE_NAME: &str = "本地插件源";
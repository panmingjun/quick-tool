//! 公共类型定义

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
    pub config: ServerConfig,
    pub auth_token: Option<String>,
    pub status: ConnectionStatus,
}
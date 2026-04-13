//! 服务端配置管理

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// 服务端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 本地唯一标识
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 服务端地址
    pub address: String,
    /// 是否为默认服务端
    pub is_default: bool,
}

impl ServerConfig {
    /// 创建新的服务端配置
    pub fn new(name: String, address: String, is_default: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            address,
            is_default,
        }
    }
}

/// 默认服务端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultServerConfig {
    pub name: String,
    pub address: String,
}

impl Default for DefaultServerConfig {
    fn default() -> Self {
        Self {
            name: "Official Quick Tool Server".to_string(),
            address: "https://api.quicktool.io".to_string(),
        }
    }
}

/// 服务端配置列表
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfigList {
    pub servers: Vec<ServerConfig>,
}

/// 加载服务端配置列表
pub fn load(config_path: PathBuf) -> qt_core::Result<ServerConfigList> {
    if !config_path.exists() {
        // 返回默认配置
        let default_server = ServerConfig::new(
            DefaultServerConfig::default().name,
            DefaultServerConfig::default().address,
            true,
        );
        return Ok(ServerConfigList {
            servers: vec![default_server],
        });
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| qt_core::Error::Config(format!("读取服务端配置失败: {}", e)))?;

    toml::from_str(&content)
        .map_err(|e| qt_core::Error::Config(format!("解析服务端配置失败: {}", e)))
}

/// 保存服务端配置列表
pub fn save(config: &ServerConfigList, config_path: PathBuf) -> qt_core::Result<()> {
    let content = toml::to_string_pretty(config)
        .map_err(|e| qt_core::Error::Config(format!("序列化服务端配置失败: {}", e)))?;

    std::fs::write(&config_path, content)
        .map_err(|e| qt_core::Error::Config(format!("写入服务端配置失败: {}", e)))?;

    Ok(())
}
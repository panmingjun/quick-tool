//! 配置管理模块

pub mod hotkey;
pub mod server;
pub mod offline;

use directories::ProjectDirs;
use std::path::PathBuf;

/// 获取配置目录路径
pub fn config_dir() -> PathBuf {
    let dir = ProjectDirs::from("io", "QuickTool", "quick-tool").map_or_else(
        || {
            tracing::warn!("无法获取配置目录，回退到当前目录");
            PathBuf::from(".")
        },
        |d| d.config_dir().to_path_buf(),
    );
    dir
}

/// 获取数据目录路径
pub fn data_dir() -> PathBuf {
    let dir = ProjectDirs::from("io", "QuickTool", "quick-tool").map_or_else(
        || {
            tracing::warn!("无法获取数据目录，回退到当前目录");
            PathBuf::from(".")
        },
        |d| d.data_dir().to_path_buf(),
    );
    dir
}
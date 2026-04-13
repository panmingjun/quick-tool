//! 配置管理模块

pub mod hotkey;
pub mod server;
pub mod offline;

use directories::ProjectDirs;
use std::path::PathBuf;

/// 获取配置目录路径
pub fn config_dir() -> PathBuf {
    let project_dirs = ProjectDirs::from("io", "QuickTool", "quick-tool")
        .expect("无法获取配置目录");
    project_dirs.config_dir().to_path_buf()
}

/// 获取数据目录路径
pub fn data_dir() -> PathBuf {
    let project_dirs = ProjectDirs::from("io", "QuickTool", "quick-tool")
        .expect("无法获取数据目录");
    project_dirs.data_dir().to_path_buf()
}
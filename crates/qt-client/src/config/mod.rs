//! 配置管理模块
//!
//! 提供配置目录、数据目录、配置文件路径的管理。

pub mod hotkey;
pub mod server;
pub mod offline;

use directories::ProjectDirs;
use std::path::PathBuf;

/// 默认配置文件名
pub const CONFIG_FILE_NAME: &str = "config.json";

/// 项目数据目录名（用于存放 SQLite 和插件数据）
pub const PROJECT_DATA_DIR: &str = "data";

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

/// 获取默认配置文件路径
///
/// 优先使用 `~/.config/quicktool/config.json`，如果该文件不存在则回退
/// 到项目目录下的 `config/config.json`。
pub fn default_config_path() -> PathBuf {
    let default_path = config_dir().join(CONFIG_FILE_NAME);
    if default_path.is_file() {
        default_path
    } else {
        // 回退到项目根目录的 config/config.json
        PathBuf::from("./config/config.json")
    }
}

/// 获取数据子目录路径（用于存放 SQLite 和插件安装目录）
pub fn data_sub_dir(sub: &str) -> PathBuf {
    data_dir().join(PROJECT_DATA_DIR).join(sub)
}

/// 获取插件安装根目录
///
/// 优先从 AppConfig 读取（如果配置了 plugin_install_dir），
/// 否则回退到 `data_dir/data/plugins`。
pub fn plugin_install_dir() -> PathBuf {
    // 先检查项目目录下的 data/plugins 是否存在
    let project_plugins = PathBuf::from("./data/plugins");
    if project_plugins.exists() {
        project_plugins
    } else {
        data_sub_dir("plugins")
    }
}

/// 获取 SQLite 数据目录
pub fn sqlite_dir() -> PathBuf {
    // 先检查项目目录下的 data/sqlite 是否存在
    let project_sqlite = PathBuf::from("./data/sqlite");
    if project_sqlite.exists() {
        project_sqlite
    } else {
        data_sub_dir("sqlite")
    }
}

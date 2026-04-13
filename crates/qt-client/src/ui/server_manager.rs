//! 服务端管理界面

use crate::config::server::ServerConfig;

/// 服务端管理状态
#[derive(Debug, Clone, Default)]
pub struct ServerManagerState {
    /// 正在编辑的服务端配置
    editing_server: Option<ServerConfig>,
    /// 是否显示添加服务端对话框
    show_add_dialog: bool,
}
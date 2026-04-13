//! 服务端列表/切换组件

use qt_core::ServerConnection;

/// 服务端列表状态
#[derive(Debug, Clone, Default)]
pub struct ServerListState {
    /// 服务端连接列表
    connections: Vec<ServerConnection>,
    /// 当前选中的服务端索引
    current_index: usize,
}
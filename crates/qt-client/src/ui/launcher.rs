//! 启动器主界面

/// 启动器状态
#[derive(Debug, Clone, Default)]
pub struct LauncherState {
    /// 是否显示
    visible: bool,
    /// 搜索关键字
    search_query: String,
    /// 当前选中的服务端
    current_server: String,
}
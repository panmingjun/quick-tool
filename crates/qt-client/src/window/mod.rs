//! 窗口管理模块

pub mod layer;

/// 窗口类型
#[derive(Debug, Clone, Copy)]
pub enum WindowType {
    /// 启动器主窗口
    Launcher,
    /// 悬浮窗
    Floating,
    /// 工具窗口
    Tool,
}
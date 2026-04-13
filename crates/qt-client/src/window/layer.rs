//! Wayland layer-shell 实现
//!
//! 用于创建悬浮窗和启动器窗口

/// Layer-shell 层级
#[derive(Debug, Clone, Copy)]
pub enum Layer {
    /// 背景层
    Background,
    /// 底部层
    Bottom,
    /// 正常层
    Normal,
    /// 顶部层 (用于悬浮窗)
    Top,
    /// 覆盖层 (用于启动器)
    Overlay,
}

/// Layer-shell 锚点
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}
//! 悬浮窗组件

/// 悬浮窗位置
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// 悬浮窗大小
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// 悬浮窗状态
#[derive(Debug, Clone)]
pub struct FloatingWindow {
    pub position: Position,
    pub size: Size,
    pub opacity: f32,
    pub always_on_top: bool,
}
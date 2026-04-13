//! 快捷键平台抽象

use crate::config::hotkey::Hotkey;
use std::sync::mpsc::Receiver;

/// 快捷键事件
#[derive(Debug, Clone)]
pub struct HotkeyEvent {
    /// 触发的快捷键
    pub hotkey: Hotkey,
}

/// 快捷键管理器 Trait
pub trait HotkeyManager: Send + Sync {
    /// 注册快捷键
    fn register(&mut self, hotkey: &Hotkey) -> qt_core::Result<()>;

    /// 取消注册快捷键
    fn unregister(&mut self, hotkey: &Hotkey) -> qt_core::Result<()>;

    /// 监听快捷键事件
    fn listen(&mut self) -> Receiver<HotkeyEvent>;
}
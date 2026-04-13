//! Wayland 全局快捷键实现
//!
//! 使用 xdg-desktop-portal 的 GlobalShortcuts 接口

use crate::config::hotkey::Hotkey;
use crate::hotkey::platform::{HotkeyEvent, HotkeyManager};
use std::sync::mpsc::{channel, Receiver};

/// Wayland 快捷键管理器
pub struct WaylandHotkeyManager {
    registered_hotkeys: Vec<Hotkey>,
}

impl WaylandHotkeyManager {
    pub fn new() -> Self {
        Self {
            registered_hotkeys: Vec::new(),
        }
    }
}

impl HotkeyManager for WaylandHotkeyManager {
    fn register(&mut self, hotkey: &Hotkey) -> qt_core::Result<()> {
        // TODO: 实现 ashpd GlobalShortcuts portal 注册
        // 首次使用需要用户在系统设置中授权
        tracing::info!("注册 Wayland 快捷键: {:?}", hotkey);
        self.registered_hotkeys.push(hotkey.clone());
        Ok(())
    }

    fn unregister(&mut self, hotkey: &Hotkey) -> qt_core::Result<()> {
        tracing::info!("取消注册 Wayland 快捷键: {:?}", hotkey);
        self.registered_hotkeys.retain(|h| h != hotkey);
        Ok(())
    }

    fn listen(&mut self) -> Receiver<HotkeyEvent> {
        let (_tx, rx) = channel();
        // TODO: 实现 portal 事件监听
        tracing::warn!("Wayland 快捷键监听尚未完全实现");
        rx
    }
}

impl Default for WaylandHotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}
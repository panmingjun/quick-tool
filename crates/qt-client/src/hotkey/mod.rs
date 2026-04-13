//! 全局快捷键模块

pub mod platform;
pub mod wayland;
pub mod x11;

pub use platform::HotkeyManager;

/// 根据当前平台创建快捷键管理器
pub fn create_manager() -> Box<dyn HotkeyManager> {
    // 检测显示服务器类型
    if is_wayland() {
        Box::new(wayland::WaylandHotkeyManager::new())
    } else {
        Box::new(x11::X11HotkeyManager::new())
    }
}

/// 检测是否为 Wayland 环境
fn is_wayland() -> bool {
    // 检查 WAYLAND_DISPLAY 或 XDG_SESSION_TYPE 环境变量
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return true;
    }
    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        return session_type == "wayland";
    }
    false
}
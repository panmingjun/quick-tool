//! 设置界面

use crate::config::hotkey::HotkeyConfig;

/// 设置状态
#[derive(Debug, Clone)]
pub struct SettingsState {
    /// 快捷键配置
    pub hotkey_config: HotkeyConfig,
}
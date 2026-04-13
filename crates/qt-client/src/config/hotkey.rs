//! 快捷键配置

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 快捷键配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// 唤起启动器快捷键 (默认: Command+Space)
    pub toggle_launcher: Hotkey,
    /// 唤起悬浮窗快捷键 (默认: Command+Shift+Space)
    pub toggle_floating: Hotkey,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            toggle_launcher: Hotkey::command_space(),
            toggle_floating: Hotkey::command_shift_space(),
        }
    }
}

/// 快捷键定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hotkey {
    pub modifiers: Vec<Modifier>,
    pub key: Key,
}

impl Hotkey {
    /// Command+Space (跨平台映射)
    pub fn command_space() -> Self {
        Self {
            modifiers: vec![Modifier::Super],
            key: Key::Space,
        }
    }

    /// Command+Shift+Space
    pub fn command_shift_space() -> Self {
        Self {
            modifiers: vec![Modifier::Super, Modifier::Shift],
            key: Key::Space,
        }
    }
}

/// 修饰键
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Super, // Win键/Mac Command/Linux Super
}

/// 按键
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Key {
    Space,
    A,
    B,
    C,
    // ... 其他按键
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

/// 加载快捷键配置
pub fn load(config_path: PathBuf) -> qt_core::Result<HotkeyConfig> {
    if !config_path.exists() {
        return Ok(HotkeyConfig::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| qt_core::Error::Config(format!("读取配置失败: {}", e)))?;

    toml::from_str(&content)
        .map_err(|e| qt_core::Error::Config(format!("解析配置失败: {}", e)))
}

/// 保存快捷键配置
pub fn save(config: &HotkeyConfig, config_path: PathBuf) -> qt_core::Result<()> {
    let content = toml::to_string_pretty(config)
        .map_err(|e| qt_core::Error::Config(format!("序列化配置失败: {}", e)))?;

    std::fs::write(&config_path, content)
        .map_err(|e| qt_core::Error::Config(format!("写入配置失败: {}", e)))?;

    Ok(())
}
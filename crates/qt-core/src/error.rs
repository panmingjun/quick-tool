//! 错误类型定义

use thiserror::Error;

/// Quick Tool 错误类型
#[derive(Debug, Error)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("WASM 运行时错误: {0}")]
    WasmRuntime(String),

    #[error("加密错误: {0}")]
    Crypto(String),

    #[error("快捷键错误: {0}")]
    Hotkey(String),

    #[error("UI 错误: {0}")]
    Ui(String),

    #[error("认证错误: {0}")]
    Auth(String),

    #[error("工具错误: {0}")]
    Tool(String),

    #[error("能力未授权: {0}")]
    CapabilityDenied(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

/// Result 类型别名
pub type Result<T> = std::result::Result<T, Error>;
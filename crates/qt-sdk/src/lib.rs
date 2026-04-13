//! WASM 工具 SDK
//!
//! 提供工具开发所需的 API 和组件

pub mod api;

pub use qt_core::{ToolMetadata, Capability, ToolId};

/// 工具 Trait - 所有 WASM 工具必须实现
pub trait Tool: Send + Sync {
    /// 工具元信息
    fn metadata(&self) -> ToolMetadata;

    /// 工具被激活时调用
    fn on_activate(&mut self);

    /// 工具被挂起时调用
    fn on_suspend(&mut self);
}
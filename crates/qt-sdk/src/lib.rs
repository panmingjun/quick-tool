//! WASM 工具 SDK
//!
//! 双端合一：宿主侧（非 wasm32）提供工具开发 API 与 UI 组件；
//! 插件侧（wasm32）提供 WIT 绑定的 guest 侧生成（`bindings` 模块）。

// 插件侧（guest）：WIT 绑定生成。插件依赖 qt-sdk 后通过
// `qt_sdk::bindings::{Guest, export}` 实现插件逻辑。
#[cfg(target_arch = "wasm32")]
pub mod bindings;

// 宿主侧（host）：工具 API 与 UI 组件仅宿主使用，插件侧不含。
#[cfg(not(target_arch = "wasm32"))]
pub mod api;

#[cfg(not(target_arch = "wasm32"))]
pub use qt_core::{Capability, ToolId, ToolMetadata};

/// 工具 Trait - 所有 WASM 工具必须实现
#[cfg(not(target_arch = "wasm32"))]
pub trait Tool: Send + Sync {
    /// 工具元信息
    fn metadata(&self) -> ToolMetadata;

    /// 工具被激活时调用
    fn on_activate(&mut self);

    /// 工具被挂起时调用
    fn on_suspend(&mut self);
}
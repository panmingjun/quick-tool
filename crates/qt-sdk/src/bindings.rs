//! 插件侧（guest）WIT 绑定
//!
//! 由 `wit-bindgen` 的 `generate!` 宏解析 `wit/plugin.wit` 生成：
//! - `Guest` trait：插件需实现的 `get_ui` / `get_state` / `dispatch_action` 接口
//! - `export!` 宏：把实现类型导出的插件组件元数据
//!
//! 插件开发方式：
//! ```rust
//! use qt_sdk::bindings::{export, Guest};
//!
//! struct MyPlugin;
//!
//! impl Guest for MyPlugin {
//!     fn get_ui() -> String {
//!         "...".to_string()
//!     }
//!     fn get_state() -> Vec<qt_sdk::bindings::qt::plugin::types::Property> {
//!         Vec::new()
//!     }
//! }
//!
//! export!(MyPlugin);
//! ```

// generate! 宏生成的 trait/类型无法逐一写文档注释，此处整体豁免 rustc missing_docs
#![allow(missing_docs)]

wit_bindgen::generate!({
    // 相对本 crate 的 Cargo.toml（`crates/qt-sdk/`）路径，即 `crates/qt-sdk/wit`
    path: "wit",
    world: "plugin",
    // 插件 crate 需要调用 `qt_sdk::bindings::export!`，故将生成的宏设为 pub
    pub_export_macro: true,
    export_macro_name: "export",
});
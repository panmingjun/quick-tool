//! WASM 插件执行层
//!
//! 提供「实例化 WASM Component → 轮询调用 get-ui / get-state → 读取模板与数据」的插件 ABI。
//! 基于 wasmtime Component Model + wit-bindgen，与 qt-sdk 中的 WIT 契约（`qt:plugin`）对应。
//! 宿主侧绑定由 `bindgen!` 依据 `crates/qt-sdk/wit/plugin.wit` 生成。

use crate::engine::WasmEngine;
use wasmtime::component::Component;

/// 数据快照中的属性-值对（WIT `types.property`），供宿主端驱动组件属性
pub use bindings::qt::plugin::types::Property;
/// 数据值变体（WIT `types.value`）
pub use bindings::qt::plugin::types::Value;

/// core wasm → component 转换后的组件实例
pub struct WasmPlugin {
    instance: bindings::Plugin,
    store: wasmtime::Store<()>,
}

/// 依据 `qt-sdk/wit/plugin.wit` 生成的宿主侧绑定
pub mod bindings {
    // bindgen! 宏生成的类型无法逐一写文档注释，此处整体豁免 rustc missing_docs
    #![allow(missing_docs)]

    wasmtime::component::bindgen!({
        // 相对本 crate 的 Cargo.toml（`crates/qt-runtime/`）路径
        path: "../../crates/qt-sdk/wit",
        world: "plugin",
    });
}

impl WasmEngine {
    /// 实例化一个插件组件：core wasm 先经 wit-component 转换为 component 再实例化
    pub fn instantiate_plugin(&self, wasm_bytes: &[u8]) -> qt_core::Result<WasmPlugin> {
        let component = encode_component(wasm_bytes)?;
        let component = Component::new(&self.engine, &component)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("组件加载失败: {e}")))?;

        let mut store = wasmtime::Store::new(&self.engine, ());
        // 为插件实例补充燃料（epoch/燃料配额由沙箱外部控制）
        store
            .set_fuel(u64::MAX)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("设置燃料失败: {e}")))?;

        // 当前插件无 import（不依赖宿主能力出口），故使用空链接器
        let linker = wasmtime::component::Linker::new(&self.engine);
        let (instance, _instance_handle) = bindings::Plugin::instantiate(&mut store, &component, &linker)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("插件实例化失败: {e}")))?;

        Ok(WasmPlugin { instance, store })
    }
}

/// 将 core wasm（含 wit-bindgen 写入的 `component-type` 自定义段）编码为 component 二进制
fn encode_component(wasm_bytes: &[u8]) -> qt_core::Result<Vec<u8>> {
    let encoder = wit_component::ComponentEncoder::default();
    let mut encoder = encoder
        .module(wasm_bytes)
        .map_err(|e| qt_core::Error::WasmRuntime(format!("组件编码失败: {e}")))?;
    encoder
        .encode()
        .map_err(|e| qt_core::Error::WasmRuntime(format!("组件编码失败: {e}")))
}

impl WasmPlugin {
    /// 轮询调用插件 `get-ui`，返回当前页面模板（.slint 源码字符串）
    pub fn get_ui(&mut self) -> qt_core::Result<String> {
        let result = self
            .instance
            .call_get_ui(&mut self.store)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("调用 get-ui 失败: {e}")))?;
        Ok(result)
    }

    /// 轮询调用插件 `get-state`，返回当前数据快照（VO）
    pub fn get_state(&mut self) -> qt_core::Result<Vec<Property>> {
        self.instance
            .call_get_state(&mut self.store)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("调用 get-state 失败: {e}")))
    }

    /// 将插件 UI 上的交互事件转发给插件，返回插件状态是否发生变更
    pub fn dispatch_action(&mut self, action: &str) -> qt_core::Result<bool> {
        self.instance
            .call_dispatch_action(&mut self.store, action)
            .map_err(|e| {
                qt_core::Error::WasmRuntime(format!("调用 dispatch-action 失败: {e}"))
            })
    }
}
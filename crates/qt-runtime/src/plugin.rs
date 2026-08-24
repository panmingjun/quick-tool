//! WASM 插件执行层
//!
//! 提供「实例化 WASM Component → 调用 get-ui / get-state / dispatch-action」的插件 ABI。
//! 基于 wasmtime Component Model + wit-bindgen，与 qt-sdk 中的 WIT 契约（`qt:plugin`）对应。
//! 宿主侧绑定由 `bindgen!` 依据 `crates/qt-sdk/wit/plugin.wit` 生成，
//! 并向插件注入 `storage` import（每插件独立 SQLite 键值存储）。

use crate::engine::WasmEngine;
use qt_storage::per_plugin::{PluginStore, DEFAULT_QUOTA_BYTES};
use wasmtime::component::Component;

/// 数据快照中的属性-值对（WIT `types.property`），供宿主端驱动组件属性
pub use bindings::qt::plugin::types::Property;
/// 数据值变体（WIT `types.value`）
pub use bindings::qt::plugin::types::Value;
/// 插件事件（WIT `types.plugin-event`）：宿主转发给插件的统一事件结构
pub use bindings::qt::plugin::types::{EventSource, PluginEvent};
/// 存储错误变体（WIT `storage.kv-error`）
pub use bindings::qt::plugin::storage::KvError;

/// 系统事件 `kind`：插件被打开（进入插件窗口后发送一次）
pub const EVENT_OPENED: &str = "opened";
/// 系统事件 `kind`：时间节拍（插件窗口活跃期间每秒一次）
pub const EVENT_TICK: &str = "tick";
/// 系统事件 `kind`：插件数据被外部变更（预留，如同步回写）
pub const EVENT_DATA_CHANGED: &str = "data-changed";

/// 构造自定义 UI 事件（`kind` 为模板回调名）
#[must_use]
pub fn custom_event(kind: &str) -> PluginEvent {
    PluginEvent {
        source: EventSource::Custom,
        kind: kind.to_string(),
        payload: None,
    }
}

/// 构造系统事件（如 `opened` / `tick` / `data-changed`）
#[must_use]
pub fn system_event(kind: &str, payload: Option<String>) -> PluginEvent {
    PluginEvent {
        source: EventSource::System,
        kind: kind.to_string(),
        payload,
    }
}

/// 插件宿主状态：向插件开放的能力出口（store data）
///
/// 作为 wasmtime `Store` 的 data 持有，WIT import 接口的 Host 实现据此
/// 访问宿主能力；当前含键值存储与窗口尺寸请求。
pub struct PluginHostState {
    /// 插件键值存储（独立 SQLite 库）；None 表示该实例未启用存储
    pub storage: Option<PluginStore>,
    /// 插件请求的窗口尺寸（逻辑像素），由 `window.set-size` 写入，
    /// 宿主在事件返回后经 `WasmPlugin::take_window_request` 取走应用
    pub window_size: Option<(u32, u32)>,
}

impl PluginHostState {
    /// 创建启用存储的宿主状态
    ///
    /// 库文件位于 `<data_root>/<plugin_id>/data.sqlite`，配额默认 10 MiB。
    pub fn with_storage(
        data_root: &std::path::Path,
        plugin_id: &str,
    ) -> qt_core::Result<Self> {
        let storage = PluginStore::open(data_root, plugin_id, DEFAULT_QUOTA_BYTES)
            .map_err(|e| qt_core::Error::Database(format!("初始化插件存储失败: {e:?}")))?;
        Ok(Self { storage: Some(storage), window_size: None })
    }
}

/// `types` 接口无函数，仅需满足 bindgen 的 Host 约束
impl bindings::qt::plugin::types::Host for PluginHostState {}

impl bindings::qt::plugin::window::Host for PluginHostState {
    fn set_size(&mut self, width: u32, height: u32) -> wasmtime::Result<()> {
        // 仅记录请求（同一次事件内以最后一次为准），由宿主在事件返回后应用
        self.window_size = Some((width, height));
        Ok(())
    }
}

impl bindings::qt::plugin::storage::Host for PluginHostState {
    fn kv_get(&mut self, key: String) -> wasmtime::Result<Option<String>> {
        let Some(store) = self.storage.as_ref() else {
            return Ok(None);
        };
        store.kv_get(&key).map_err(wasmtime_err)
    }

    fn kv_set(&mut self, key: String, value: String) -> wasmtime::Result<Result<(), KvError>> {
        let Some(store) = self.storage.as_mut() else {
            return Ok(Err(KvError::IoError("插件未启用存储".to_string())));
        };
        match store.kv_set(&key, &value) {
            Ok(()) => Ok(Ok(())),
            Err(qt_storage::per_plugin::KvError::QuotaExceeded(quota)) => {
                Ok(Err(KvError::QuotaExceeded(quota)))
            }
            Err(qt_storage::per_plugin::KvError::Io(msg)) => Err(anyhow::anyhow!(msg)),
        }
    }

    fn kv_delete(&mut self, key: String) -> wasmtime::Result<Result<(), KvError>> {
        let Some(store) = self.storage.as_mut() else {
            return Ok(Err(KvError::IoError("插件未启用存储".to_string())));
        };
        store
            .kv_delete(&key)
            .map(Ok)
            .map_err(|e| anyhow::anyhow!("{e:?}"))
    }

    fn kv_keys(&mut self) -> wasmtime::Result<Result<Vec<String>, KvError>> {
        let Some(store) = self.storage.as_ref() else {
            return Ok(Ok(Vec::new()));
        };
        store.kv_keys().map(Ok).map_err(wasmtime_err)
    }

    fn used_bytes(&mut self) -> wasmtime::Result<u64> {
        let Some(store) = self.storage.as_ref() else {
            return Ok(0);
        };
        store.used_bytes().map_err(wasmtime_err)
    }
}

/// 把存储错误转换为 wasmtime trap 错误
fn wasmtime_err(e: qt_storage::per_plugin::KvError) -> anyhow::Error {
    anyhow::anyhow!("{e:?}")
}

/// core wasm → component 转换后的组件实例
pub struct WasmPlugin {
    instance: bindings::Plugin,
    store: wasmtime::Store<PluginHostState>,
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
    ///
    /// `host_state` 提供插件可调用的宿主能力（如键值存储），随 Store 生命周期管理；
    /// 返回列表丢弃 `WasmPlugin` 时一并释放，杜绝后台残留。
    pub fn instantiate_plugin(
        &self,
        wasm_bytes: &[u8],
        host_state: PluginHostState,
    ) -> qt_core::Result<WasmPlugin> {
        let component = encode_component(wasm_bytes)?;
        let component = Component::new(&self.engine, &component)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("组件加载失败: {e}")))?;

        let mut store = wasmtime::Store::new(&self.engine, host_state);
        // 为插件实例补充燃料（epoch/燃料配额由沙箱外部控制）
        store
            .set_fuel(u64::MAX)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("设置燃料失败: {e}")))?;

        // 注入 WIT world 声明的 import 接口（storage 等宿主能力出口）
        let mut linker = wasmtime::component::Linker::new(&self.engine);
        bindings::Plugin::add_to_linker(&mut linker, |state: &mut PluginHostState| state)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("注册宿主能力失败: {e}")))?;

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
    /// 调用插件 `get-ui`，返回当前页面模板（.slint 源码字符串）
    pub fn get_ui(&mut self) -> qt_core::Result<String> {
        let result = self
            .instance
            .call_get_ui(&mut self.store)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("调用 get-ui 失败: {e}")))?;
        Ok(result)
    }

    /// 调用插件 `get-state`，返回当前数据快照（VO）
    pub fn get_state(&mut self) -> qt_core::Result<Vec<Property>> {
        self.instance
            .call_get_state(&mut self.store)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("调用 get-state 失败: {e}")))
    }

    /// 将统一事件转发给插件（系统事件 + 自定义 UI 事件），
    /// 返回插件状态是否发生变更
    pub fn dispatch_event(&mut self, event: &PluginEvent) -> qt_core::Result<bool> {
        self.instance
            .call_dispatch_event(&mut self.store, event)
            .map_err(|e| {
                qt_core::Error::WasmRuntime(format!("调用 dispatch-event 失败: {e}"))
            })
    }

    /// 取走插件在本次事件中请求的窗口尺寸（逻辑像素）
    ///
    /// 由宿主在 `dispatch_event` 返回后调用并应用到插件窗口；
    /// 无请求时返回 None。
    pub fn take_window_request(&mut self) -> Option<(u32, u32)> {
        self.store.data_mut().window_size.take()
    }
}
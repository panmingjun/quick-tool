//! Wasmtime 引擎封装

use wasmtime::*;

/// WASM 引擎配置
pub struct WasmEngine {
    engine: Engine,
    linker: Linker<HostState>,
}

/// 主机状态
pub struct HostState {
    /// 能力授权
    capabilities: Vec<qt_core::Capability>,
}

impl WasmEngine {
    /// 创建新的 WASM 引擎
    pub fn new() -> qt_core::Result<Self> {
        let mut config = Config::new();
        config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
        config.wasm_bulk_memory(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("引擎创建失败: {}", e)))?;

        let linker = Linker::new(&engine);

        Ok(Self { engine, linker })
    }

    /// 加载 WASM 模块
    pub fn load_module(&self, wasm_bytes: &[u8]) -> qt_core::Result<Module> {
        Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("模块加载失败: {}", e)))
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new().expect("WASM 引擎初始化失败")
    }
}
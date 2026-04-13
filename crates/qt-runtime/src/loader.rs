//! WASM 模块加载器

use crate::engine::WasmEngine;
use std::path::Path;

/// WASM 模块加载器
pub struct ModuleLoader {
    engine: WasmEngine,
}

impl ModuleLoader {
    /// 创建新加载器
    pub fn new(engine: WasmEngine) -> Self {
        Self { engine }
    }

    /// 从文件加载模块
    pub fn load_from_file(&self, path: &Path) -> qt_core::Result<Vec<u8>> {
        std::fs::read(path)
            .map_err(|e| qt_core::Error::WasmRuntime(format!("读取 WASM 文件失败: {}", e)))
    }

    /// 从内存加载模块
    pub fn load_from_bytes(&self, bytes: &[u8]) -> qt_core::Result<LoadedModule> {
        let module = self.engine.load_module(bytes)?;
        Ok(LoadedModule { module })
    }
}

/// 已加载的模块
pub struct LoadedModule {
    module: wasmtime::Module,
}
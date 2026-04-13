//! 工具实例管理

use crate::engine::WasmEngine;
use crate::loader::LoadedModule;
use crate::sandbox::Sandbox;
use qt_core::ToolId;

/// 工具实例
pub struct ToolInstance {
    /// 实例 ID
    pub id: ToolId,
    /// 模块
    module: LoadedModule,
    /// 沙箱
    sandbox: Sandbox,
}

/// 实例管理器
pub struct InstanceManager {
    instances: Vec<ToolInstance>,
    engine: WasmEngine,
}

impl InstanceManager {
    /// 创建新管理器
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            engine: WasmEngine::new().expect("WASM 引擎初始化失败"),
        }
    }

    /// 创建新实例
    pub fn create_instance(&mut self, module: LoadedModule, sandbox: Sandbox) -> ToolId {
        let id = ToolId::new();
        let instance = ToolInstance {
            id: id.clone(),
            module,
            sandbox,
        };
        self.instances.push(instance);
        id
    }

    /// 获取实例
    pub fn get_instance(&self, id: &ToolId) -> Option<&ToolInstance> {
        self.instances.iter().find(|i| &i.id == id)
    }

    /// 销毁实例
    pub fn destroy_instance(&mut self, id: &ToolId) {
        self.instances.retain(|i| &i.id != id);
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}
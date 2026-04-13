//! 沙箱隔离层

use qt_core::Capability;

/// 工具沙箱配置
pub struct SandboxConfig {
    /// 允许的能力
    pub capabilities: Vec<Capability>,
    /// 数据存储路径限制
    pub data_path: Option<String>,
    /// 网络访问白名单
    pub network_whitelist: Vec<String>,
}

/// 沙箱实例
pub struct Sandbox {
    config: SandboxConfig,
}

impl Sandbox {
    /// 创建新沙箱
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// 检查能力是否授权
    pub fn check_capability(&self, capability: &Capability) -> bool {
        self.config.capabilities.contains(capability)
    }

    /// 检查网络访问是否允许
    pub fn check_network_access(&self, url: &str) -> bool {
        if self.config.network_whitelist.is_empty() {
            return true;
        }
        self.config.network_whitelist.iter().any(|allowed| url.starts_with(allowed))
    }
}
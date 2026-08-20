//! 插件接入规范类型
//!
//! 参考 uTools 的 `plugin.json` 结构（`main` / `features{code, explain, cmds}`），
//! 适配 WASM 运行引擎：`main` 指向插件 WASM 入口，`engine` 标识运行引擎。

use serde::{Deserialize, Serialize};

/// 插件接入规范默认运行引擎
const DEFAULT_ENGINE: &str = "wasm";

/// 插件清单（对应插件目录下的 `plugin.json`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// 插件唯一标识
    pub id: String,
    /// 插件显示名称
    pub name: String,
    /// 版本号
    #[serde(default)]
    pub version: String,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 功能描述
    #[serde(default)]
    pub description: String,
    /// 运行入口：WASM 文件路径（相对 `plugin.json` 所在目录）
    #[serde(default)]
    pub main: String,
    /// 图标路径（可选）
    #[serde(default)]
    pub logo: Option<String>,
    /// 运行引擎标识（默认 `wasm`）
    #[serde(default = "default_engine")]
    pub engine: String,
    /// 功能指令集合
    #[serde(default)]
    pub features: Vec<PluginFeature>,
}

impl PluginManifest {
    /// 从 JSON 字符串解析插件清单
    pub fn from_json(json: &str) -> crate::Result<Self> {
        serde_json::from_str(json).map_err(|e| crate::Error::Config(format!("解析 plugin.json 失败: {e}")))
    }

    /// 检查清单是否适配指定运行引擎
    pub fn matches_engine(&self, engine: &str) -> bool {
        self.engine.is_empty() || self.engine == engine
    }
}

/// 默认运行引擎
fn default_engine() -> String {
    DEFAULT_ENGINE.to_string()
}

/// 插件功能点（对应 `features` 数组的元素）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFeature {
    /// 功能指令编码
    pub code: String,
    /// 功能说明
    #[serde(default)]
    pub explain: String,
    /// 匹配指令（关键词，用于唤起）
    #[serde(default)]
    pub cmds: Vec<String>,
}
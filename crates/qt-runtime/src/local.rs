//! 本地插件发现与加载
//!
//! 从本地目录扫描已安装插件：每个插件是一个子目录，
//! 目录内必须有 `plugin.json`（规范见 `docs/plugin-spec.md`）。

use qt_core::PluginManifest;
use std::path::{Path, PathBuf};

/// 本地插件
#[derive(Debug, Clone)]
pub struct LocalPlugin {
    /// 插件清单
    pub manifest: PluginManifest,
    /// 插件根目录（`plugin.json` 所在目录）
    pub root_dir: PathBuf,
}

impl LocalPlugin {
    /// 解析 WASM 入口文件路径
    pub fn wasm_path(&self) -> qt_core::Result<PathBuf> {
        if self.manifest.main.is_empty() {
            return Err(qt_core::Error::Tool(
                "plugin.json 未配置 main 入口".to_string(),
            ));
        }

        let main_path = PathBuf::from(&self.manifest.main);
        let path = if main_path.is_absolute() {
            main_path
        } else {
            self.root_dir.join(&main_path)
        };

        std::fs::canonicalize(&path)
            .map_err(|e| qt_core::Error::Tool(format!("解析插件入口失败: {e}")))
    }
}

/// 扫描本地插件目录，返回其中所有 `engine == wasm` 的插件
pub fn discover_plugins(plugins_dir: &Path) -> qt_core::Result<Vec<LocalPlugin>> {
    let mut plugins = Vec::new();
    if !plugins_dir.is_dir() {
        return Ok(plugins);
    }

    let entries = std::fs::read_dir(plugins_dir)
        .map_err(|e| qt_core::Error::Config(format!("读取插件目录失败: {e}")))?;

    for entry in entries {
        let dir = entry
            .map_err(|e| qt_core::Error::Config(format!("读取插件目录项失败: {e}")))?
            .path();
        if !dir.is_dir() {
            continue;
        }

        let manifest_path = dir.join("plugin.json");
        if !manifest_path.is_file() {
            continue;
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| qt_core::Error::Config(format!("读取插件清单失败: {e}")))?;
        let manifest = PluginManifest::from_json(&content)?;

        // 只加载当前引擎（WASM）可运行的插件
        if !manifest.matches_engine(ENGINE_WASM) {
            continue;
        }

        plugins.push(LocalPlugin {
            manifest,
            root_dir: dir,
        });
    }

    Ok(plugins)
}

/// 当前支持的插件运行引擎
pub const ENGINE_WASM: &str = "wasm";
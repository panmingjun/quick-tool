//! 插件源注册表
//!
//! 负责管理多个插件源（本地 + 远程），包括加载插件源列表、
//! 解析插件源 JSON、发现已安装的插件。

use qt_core::{AppConfig, PluginList, PluginSource, PluginSourceKind};
use std::path::{Path, PathBuf};

/// 插件注册表
///
/// 管理多个插件源，提供插件发现接口。
pub struct PluginRegistry {
    /// 应用配置（含插件源列表）
    config: AppConfig,
    /// 插件安装根目录
    install_root: PathBuf,
    /// 配置文件所在目录（用于解析相对路径）
    config_dir: PathBuf,
}

impl PluginRegistry {
    /// 从配置文件创建插件注册表
    pub fn from_config(config: AppConfig, install_root: PathBuf) -> Self {
        Self {
            config,
            install_root,
            config_dir: PathBuf::from("."),
        }
    }

    /// 从文件路径加载配置并创建注册表
    ///
    /// 如果指定路径不存在，自动回退到 `./config/config.json`。
    pub fn load(config_path: &Path, install_root: PathBuf) -> qt_core::Result<Self> {
        let config_dir = config_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        // 先尝试加载指定路径，不存在则回退到项目目录
        let config = match AppConfig::from_file(config_path) {
            Ok(c) => c,
            Err(_) => {
                let fallback = PathBuf::from("./config/config.json");
                tracing::warn!(
                    "配置文件 {} 不存在或加载失败，回退到 {}",
                    config_path.display(),
                    fallback.display()
                );
                AppConfig::from_file(&fallback).map_err(|e| {
                    qt_core::Error::Config(format!(
                        "配置文件 {} 和回退配置 {} 均加载失败: {e}",
                        config_path.display(),
                        fallback.display()
                    ))
                })?
            }
        };

        Ok(Self {
            config,
            install_root,
            config_dir,
        })
    }

    /// 保存当前配置
    pub fn save(&self, config_path: &Path) -> qt_core::Result<()> {
        self.config.to_file(config_path)
    }

    /// 解析插件源的 url 字段为实际路径
    ///
    /// 如果是绝对路径则直接返回；否则相对于当前工作目录（CWD）解析。
    /// 因为 config.json 本身可能位于 `~/.config/quicktool/`，但插件路径
    /// 应该相对于 CWD（通常是项目根目录）。
    pub fn resolve_source_url(&self, source: &PluginSource) -> PathBuf {
        let url = &source.url;
        if Path::new(url).is_absolute() {
            PathBuf::from(url)
        } else {
            PathBuf::from(url)
        }
    }

    /// 获取插件源列表
    pub fn sources(&self) -> &[PluginSource] {
        &self.config.plugin_sources.sources
    }

    /// 获取已启用的插件源
    pub fn enabled_sources(&self) -> Vec<&PluginSource> {
        self.config
            .plugin_sources
            .sources
            .iter()
            .filter(|s| s.enabled)
            .collect()
    }

    /// 添加一个插件源
    pub fn add_source(&mut self, source: PluginSource) {
        // 检查是否已存在相同 ID
        if let Some(existing) = self
            .config
            .plugin_sources
            .sources
            .iter_mut()
            .find(|s| s.id == source.id)
        {
            *existing = source;
        } else {
            self.config.plugin_sources.sources.push(source);
        }
    }

    /// 移除一个插件源
    pub fn remove_source(&mut self, source_id: &str) {
        self.config
            .plugin_sources
            .sources
            .retain(|s| s.id != source_id);
    }

    /// 从本地插件目录发现已安装的插件
    pub fn discover_plugins(&self) -> qt_core::Result<Vec<LocalPluginInfo>> {
        let mut plugins = Vec::new();

        for source in self.enabled_sources() {
            match &source.kind {
                PluginSourceKind::Local => {
                    let source_dir = Path::new(&source.url);
                    if !source_dir.is_dir() {
                        tracing::warn!("本地插件源目录不存在: {url}", url = source.url);
                        continue;
                    }
                    let found = discover_plugins_from_source(source_dir, &source.id)?;
                    plugins.extend(found);
                }
                PluginSourceKind::Remote => {
                    // 从本地安装目录查找已缓存的远程插件
                    let cached_dir = self.install_root.join(&source.id);
                    if cached_dir.is_dir() {
                        let found = discover_plugins_from_source(&cached_dir, &source.id)?;
                        plugins.extend(found);
                    }
                }
            }
        }

        Ok(plugins)
    }

    /// 获取插件安装目录
    pub fn install_root(&self) -> &Path {
        &self.install_root
    }
}

/// 已发现的插件信息（统一了本地和远程缓存的插件）
#[derive(Debug, Clone)]
pub struct LocalPluginInfo {
    /// 插件 ID
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// 所属插件源 ID
    pub source_id: String,
    /// WASM 文件路径
    pub wasm_path: PathBuf,
    /// 插件根目录
    pub root_dir: PathBuf,
}

/// 从指定目录扫描插件（每个子目录是一个插件）
pub fn discover_plugins_from_source(
    source_dir: &Path,
    source_id: &str,
) -> qt_core::Result<Vec<LocalPluginInfo>> {
    let mut plugins = Vec::new();

    if !source_dir.is_dir() {
        return Ok(plugins);
    }

    let entries = std::fs::read_dir(source_dir)
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
        let manifest =
            qt_core::PluginManifest::from_json(&content).map_err(|e| {
                qt_core::Error::PluginSource(format!("解析插件清单失败 [{dir}]: {e}", dir = dir.display()))
            })?;

        // 只加载 WASM 引擎插件
        if !manifest.matches_engine("wasm") {
            continue;
        }

        let wasm_path = dir.join(&manifest.main)
            .canonicalize()
            .unwrap_or_else(|_| dir.join(&manifest.main));

        plugins.push(LocalPluginInfo {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            author: manifest.author,
            description: manifest.description,
            source_id: source_id.to_string(),
            wasm_path,
            root_dir: dir,
        });
    }

    Ok(plugins)
}

/// 加载插件源 JSON 文件，返回插件列表
pub fn load_plugin_list(file_path: &Path) -> qt_core::Result<PluginList> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| qt_core::Error::PluginSource(format!("读取插件列表失败: {e}")))?;
    let plugin_list: PluginList = serde_json::from_str(&content)
        .map_err(|e| qt_core::Error::PluginSource(format!("解析插件列表失败: {e}")))?;
    Ok(plugin_list)
}

/// 从远程 URL 加载插件列表
///
/// 使用 reqwest 发起 HTTP GET 请求获取远程 JSON。
#[cfg(feature = "http")]
pub async fn load_remote_plugin_list(url: &str) -> qt_core::Result<PluginList> {
    let client = reqwest::Client::new();
    let content = client
        .get(url)
        .send()
        .await
        .map_err(|e| qt_core::Error::PluginSource(format!("请求远程插件列表失败: {e}")))?
        .text()
        .await
        .map_err(|e| qt_core::Error::PluginSource(format!("读取远程响应失败: {e}")))?;

    let plugin_list: PluginList = serde_json::from_str(&content)
        .map_err(|e| qt_core::Error::PluginSource(format!("解析远程插件列表失败: {e}")))?;
    Ok(plugin_list)
}

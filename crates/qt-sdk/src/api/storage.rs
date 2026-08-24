//! 数据存储 API
//!
//! 与 WIT `qt:plugin.storage` 接口对应的宿主侧语义定义（key-value，值均为 UTF-8 字符串）。
//! 实际的插件↔宿主桥接由 `bindings`（WIT 生成）+ `qt-storage::per_plugin::PluginStore` 完成。

/// 存储操作接口（key-value，按插件隔离）
pub trait StorageApi {
    /// 获取数据；不存在返回 None
    fn get(&self, key: &str) -> qt_core::Result<Option<String>>;

    /// 设置数据（存在则覆盖）；超配额时返回错误
    fn set(&mut self, key: &str, value: &str) -> qt_core::Result<()>;

    /// 删除数据；键不存在视为成功
    fn delete(&mut self, key: &str) -> qt_core::Result<()>;

    /// 获取所有键
    fn keys(&self) -> qt_core::Result<Vec<String>>;
}

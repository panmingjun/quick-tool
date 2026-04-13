//! 数据存储 API

/// 存储操作接口
pub trait StorageApi {
    /// 获取数据
    fn get(&self, key: &str) -> qt_core::Result<Option<Vec<u8>>>;

    /// 设置数据
    fn set(&self, key: &str, value: &[u8]) -> qt_core::Result<()>;

    /// 删除数据
    fn delete(&self, key: &str) -> qt_core::Result<()>;

    /// 获取所有键
    fn keys(&self) -> qt_core::Result<Vec<String>>;
}
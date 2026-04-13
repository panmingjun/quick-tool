//! 剪贴板 API

/// 剪贴板操作接口
pub trait ClipboardApi {
    /// 获取剪贴板文本
    fn get_text(&self) -> qt_core::Result<Option<String>>;

    /// 设置剪贴板文本
    fn set_text(&self, text: &str) -> qt_core::Result<()>;

    /// 获取剪贴板图片
    fn get_image(&self) -> qt_core::Result<Option<Vec<u8>>>;
}
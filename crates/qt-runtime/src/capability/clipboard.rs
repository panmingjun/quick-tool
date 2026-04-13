//! 剪贴板能力

/// 剪贴板能力实现
pub struct ClipboardCapability {
    /// 是否允许读取
    allow_read: bool,
    /// 是否允许写入
    allow_write: bool,
}

impl ClipboardCapability {
    pub fn new(allow_read: bool, allow_write: bool) -> Self {
        Self { allow_read, allow_write }
    }
}
//! 屏幕访问能力

/// 屏幕能力实现
pub struct ScreenCapability {
    /// 是否允许获取屏幕信息
    allow_screen_info: bool,
    /// 是否允许截图
    allow_capture: bool,
}

impl ScreenCapability {
    pub fn new(allow_screen_info: bool, allow_capture: bool) -> Self {
        Self { allow_screen_info, allow_capture }
    }
}
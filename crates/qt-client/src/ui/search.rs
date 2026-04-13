//! 搜索框组件

/// 搜索状态
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// 当前搜索关键字
    query: String,
    /// 是否聚焦
    focused: bool,
}
//! 工具列表组件

use qt_core::ToolMetadata;

/// 工具列表状态
#[derive(Debug, Clone, Default)]
pub struct ToolListState {
    /// 工具列表
    tools: Vec<ToolMetadata>,
    /// 当前选中的工具索引
    selected_index: Option<usize>,
}
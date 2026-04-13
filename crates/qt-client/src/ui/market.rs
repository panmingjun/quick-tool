//! 工具市场 UI

use qt_core::ToolMetadata;

/// 工具市场状态
#[derive(Debug, Clone, Default)]
pub struct MarketState {
    /// 可安装的工具列表
    available_tools: Vec<ToolMetadata>,
    /// 搜索关键字
    search_query: String,
}
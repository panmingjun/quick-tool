//! Markdown 记事本工具
//!
//! 可以独立运行用于调试

use qt_sdk::Tool;
use qt_core::{ToolMetadata, ToolId, Capability};

/// Markdown 记事本
#[allow(dead_code)]
pub struct MarkdownNote {
    id: ToolId,
    content: String,
}

impl MarkdownNote {
    pub fn new() -> Self {
        Self {
            id: ToolId::new(),
            content: String::new(),
        }
    }
}

impl Tool for MarkdownNote {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: self.id.clone(),
            name: "Markdown 记事本".to_string(),
            description: "一个简单的 Markdown 编辑器".to_string(),
            version: "1.0.0".to_string(),
            author: "Quick Tool Team".to_string(),
            keywords: vec!["markdown".to_string(), "note".to_string(), "editor".to_string()],
            capabilities: vec![Capability::Storage],
        }
    }

    fn on_activate(&mut self) {
        tracing::info!("Markdown 记事本激活");
    }

    fn on_suspend(&mut self) {
        tracing::info!("Markdown 记事本挂起");
    }
}

impl Default for MarkdownNote {
    fn default() -> Self {
        Self::new()
    }
}

/// 独立调试入口
fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Markdown 记事本工具独立调试模式");

    let mut tool = MarkdownNote::new();
    tracing::info!("工具元数据: {:?}", tool.metadata());

    tool.on_activate();
    tracing::info!("工具已激活，等待调试...");

    // 模拟运行
    tool.on_suspend();
    tracing::info!("工具已挂起");
}
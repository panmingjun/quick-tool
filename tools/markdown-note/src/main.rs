//! Markdown 记事本工具独立调试入口

use markdown_note::MarkdownNote;
use qt_sdk::Tool;

fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("=== Markdown 记事本工具独立调试 ===");

    let mut tool = MarkdownNote::new();
    tracing::info!("工具元数据:");
    tracing::info!("  ID: {}", tool.metadata().id.0);
    tracing::info!("  名称: {}", tool.metadata().name);
    tracing::info!("  版本: {}", tool.metadata().version);
    tracing::info!("  能力: {:?}", tool.metadata().capabilities);

    tool.on_activate();
    tracing::info!("工具已激活");

    // 模拟操作
    std::thread::sleep(std::time::Duration::from_secs(1));

    tool.on_suspend();
    tracing::info!("工具已挂起");

    tracing::info!("=== 调试结束 ===");
}
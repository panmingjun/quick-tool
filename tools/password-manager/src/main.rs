//! 密码管理器工具独立调试入口

use password_manager::PasswordManager;
use qt_sdk::Tool;

fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("=== 密码管理器工具独立调试 ===");

    let mut tool = PasswordManager::new();
    tracing::info!("工具元数据:");
    tracing::info!("  ID: {}", tool.metadata().id.0);
    tracing::info!("  名称: {}", tool.metadata().name);
    tracing::info!("  版本: {}", tool.metadata().version);
    tracing::info!("  能力: {:?}", tool.metadata().capabilities);

    // 测试加密功能
    tracing::info!("测试加密功能...");
    tool.set_master_password("my_master_password_123");

    if let Some((encrypted, nonce)) = tool.encrypt_password("my_secret_password") {
        tracing::info!("加密成功: {} bytes", encrypted.len());

        if let Some(decrypted) = tool.decrypt_password(&encrypted, &nonce) {
            tracing::info!("解密成功: {}", decrypted);
        } else {
            tracing::error!("解密失败");
        }
    } else {
        tracing::error!("加密失败");
    }

    tool.on_activate();
    tracing::info!("工具已激活");

    std::thread::sleep(std::time::Duration::from_secs(1));

    tool.on_suspend();
    tracing::info!("工具已挂起");

    tracing::info!("=== 调试结束 ===");
}
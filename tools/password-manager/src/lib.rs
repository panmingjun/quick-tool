//! 密码管理器工具
//!
//! 在 WASM 内完成所有加密操作，宿主无法访问明文密码

use qt_sdk::Tool;
use qt_core::{ToolMetadata, ToolId, Capability};
use serde::{Deserialize, Serialize};

/// 密码条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub name: String,
    pub username: String,
    /// 加密后的密码
    pub encrypted_password: Vec<u8>,
    pub nonce: [u8; 12],
    pub url: Option<String>,
    pub notes: Option<String>,
}

/// 密码管理器
#[allow(dead_code)]
pub struct PasswordManager {
    id: ToolId,
    /// 主密码派生的密钥 (在 WASM 内)
    master_key: Option<[u8; 32]>,
    /// 密码条目列表 (加密存储)
    entries: Vec<PasswordEntry>,
}

impl PasswordManager {
    pub fn new() -> Self {
        Self {
            id: ToolId::new(),
            master_key: None,
            entries: Vec::new(),
        }
    }

    /// 设置主密码并派生密钥
    pub fn set_master_password(&mut self, password: &str) {
        let salt = qt_core::crypto::generate_salt(16);
        self.master_key = qt_core::crypto::derive_key(password, &salt).ok();
    }

    /// 加密密码
    pub fn encrypt_password(&self, password: &str) -> Option<(Vec<u8>, [u8; 12])> {
        self.master_key.as_ref().and_then(|key| {
            qt_core::crypto::encrypt(key, password.as_bytes()).ok()
        })
    }

    /// 解密密码
    pub fn decrypt_password(&self, encrypted: &[u8], nonce: &[u8; 12]) -> Option<String> {
        self.master_key.as_ref().and_then(|key| {
            qt_core::crypto::decrypt(key, nonce, encrypted).ok()
        }).and_then(|bytes| String::from_utf8(bytes).ok())
    }
}

impl Tool for PasswordManager {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: self.id.clone(),
            name: "密码管理器".to_string(),
            description: "安全的密码管理工具，所有加密在 WASM 内完成".to_string(),
            version: "1.0.0".to_string(),
            author: "Quick Tool Team".to_string(),
            keywords: vec!["password".to_string(), "security".to_string(), "encryption".to_string()],
            capabilities: vec![Capability::Storage, Capability::Clipboard],
        }
    }

    fn on_activate(&mut self) {
        tracing::info!("密码管理器激活");
    }

    fn on_suspend(&mut self) {
        tracing::info!("密码管理器挂起");
    }
}

impl Default for PasswordManager {
    fn default() -> Self {
        Self::new()
    }
}
//! 加密工具模块
//!
//! 提供 AES-256-GCM 加密和 Argon2 密码派生

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::Error;

/// 加密密钥 (32 bytes for AES-256)
pub type EncryptionKey = [u8; 32];

/// 加密 nonce (12 bytes for GCM)
pub type EncryptionNonce = [u8; 12];

/// 使用 Argon2id 从密码派生加密密钥
pub fn derive_key(password: &str, salt: &[u8]) -> crate::Result<EncryptionKey> {
    let params = Params::new(
        Params::DEFAULT_M_COST,
        Params::DEFAULT_T_COST,
        Params::DEFAULT_P_COST,
        Some(32),
    )
    .map_err(|e| Error::Crypto(format!("Argon2 参数错误: {}", e)))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| Error::Crypto(format!("密钥派生失败: {}", e)))?;

    Ok(key)
}

/// 使用 AES-256-GCM 加密数据
pub fn encrypt(key: &EncryptionKey, plaintext: &[u8]) -> crate::Result<(Vec<u8>, EncryptionNonce)> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::Crypto(format!("Cipher 创建失败: {}", e)))?;

    let nonce = generate_nonce();
    let ciphertext = cipher
        .encrypt(&Nonce::from(nonce), plaintext)
        .map_err(|e| Error::Crypto(format!("加密失败: {}", e)))?;

    Ok((ciphertext, nonce))
}

/// 使用 AES-256-GCM 解密数据
pub fn decrypt(key: &EncryptionKey, nonce: &EncryptionNonce, ciphertext: &[u8]) -> crate::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::Crypto(format!("Cipher 创建失败: {}", e)))?;

    let plaintext = cipher
        .decrypt(&Nonce::from(*nonce), ciphertext)
        .map_err(|e| Error::Crypto(format!("解密失败: {}", e)))?;

    Ok(plaintext)
}

/// 生成随机 nonce
fn generate_nonce() -> EncryptionNonce {
    use rand::RngCore;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

/// 生成随机 salt
pub fn generate_salt(size: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut salt = vec![0u8; size];
    OsRng.fill_bytes(&mut salt);
    salt
}
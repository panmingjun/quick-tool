//! 加密 API (在 WASM 内使用)

/// 加密操作接口
pub trait CryptoApi {
    /// 使用 Argon2id 从密码派生密钥
    fn derive_key(&self, password: &str, salt: &[u8]) -> qt_core::Result<[u8; 32]>;

    /// AES-256-GCM 加密
    fn encrypt(&self, key: &[u8; 32], plaintext: &[u8]) -> qt_core::Result<(Vec<u8>, [u8; 12])>;

    /// AES-256-GCM 解密
    fn decrypt(&self, key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> qt_core::Result<Vec<u8>>;

    /// 生成随机 salt
    fn generate_salt(&self, size: usize) -> Vec<u8>;
}
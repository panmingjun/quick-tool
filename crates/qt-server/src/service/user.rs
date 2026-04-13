//! 用户服务

use crate::db::DbPool;
use qt_core::UserId;
use uuid::Uuid;

/// 用户服务
pub struct UserService {
    pool: DbPool,
}

impl UserService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// 注册用户
    pub async fn register(&self, _username: &str, _email: &str, _password: &str) -> qt_core::Result<UserId> {
        // TODO: 实现密码哈希和用户创建
        Ok(UserId(Uuid::new_v4()))
    }

    /// 验证用户登录
    pub async fn verify_login(&self, _username: &str, _password: &str) -> qt_core::Result<Option<UserId>> {
        // TODO: 实现登录验证
        Ok(None)
    }
}
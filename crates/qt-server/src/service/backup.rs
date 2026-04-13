//! 备份服务

use crate::db::DbPool;
use qt_core::UserId;

/// 备份服务
pub struct BackupService {
    pool: DbPool,
}

impl BackupService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// 上传备份
    pub async fn upload(&self, _user_id: &UserId, _data_type: &str, _encrypted_data: &[u8]) -> qt_core::Result<()> {
        // TODO: 实现备份上传
        Ok(())
    }

    /// 下载备份
    pub async fn download(&self, _user_id: &UserId, _backup_id: &str) -> qt_core::Result<Vec<u8>> {
        // TODO: 实现备份下载
        Ok(vec![])
    }
}
//! 工具服务

use crate::db::DbPool;
use qt_core::ToolMetadata;

/// 工具服务
pub struct ToolService {
    pool: DbPool,
}

impl ToolService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// 获取工具列表
    pub async fn list(&self) -> qt_core::Result<Vec<ToolMetadata>> {
        // TODO: 实现工具列表查询
        Ok(vec![])
    }

    /// 获取工具 WASM 数据
    pub async fn get_wasm(&self, _tool_id: &str) -> qt_core::Result<Vec<u8>> {
        // TODO: 实现 WASM 数据获取
        Ok(vec![])
    }
}
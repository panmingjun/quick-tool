//! 离线模式管理
//!
//! 支持客户端离线启动，用于调试和断网场景

use serde::{Deserialize, Serialize};

/// 离线模式状态
#[derive(Debug, Clone, Default)]
pub struct OfflineState {
    /// 是否处于离线模式
    is_offline: bool,
    /// 离线模式下暂存的数据变更（退出离线模式时同步）
    pending_sync: PendingSyncData,
}

/// 暂存的数据变更
#[derive(Debug, Clone, Default)]
pub struct PendingSyncData {
    /// 待同步的工具配置变更
    pub tool_configs: Vec<ToolConfigChange>,
    /// 待同步的数据变更
    pub data_changes: Vec<DataChange>,
}

/// 工具配置变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfigChange {
    pub tool_id: String,
    pub server_id: String,
    pub config: Vec<u8>,
    pub changed_at: chrono::DateTime<chrono::Utc>,
}

/// 数据变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChange {
    pub id: String,
    pub server_id: String,
    pub data_type: String,
    pub encrypted_blob: Vec<u8>,
    pub changed_at: chrono::DateTime<chrono::Utc>,
}

/// 插件同步控制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSyncControl {
    /// 工具 ID
    pub tool_id: String,
    /// 是否启用自动同步
    pub sync_enabled: bool,
    /// 最后同步时间
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl OfflineState {
    /// 进入离线模式
    pub fn enter_offline(&mut self) {
        self.is_offline = true;
        tracing::info!("进入离线模式");
    }

    /// 退出离线模式并触发同步
    pub fn exit_offline(&mut self) -> PendingSyncData {
        self.is_offline = false;
        tracing::info!("退出离线模式，准备同步数据");
        std::mem::take(&mut self.pending_sync)
    }

    /// 检查是否处于离线模式
    pub fn is_offline(&self) -> bool {
        self.is_offline
    }

    /// 记录数据变更
    pub fn record_change(&mut self, change: DataChange) {
        if self.is_offline {
            self.pending_sync.data_changes.push(change);
        }
    }

    /// 记录工具配置变更
    pub fn record_config_change(&mut self, change: ToolConfigChange) {
        if self.is_offline {
            self.pending_sync.tool_configs.push(change);
        }
    }
}
# 离线模式说明

## 功能限制

离线模式下以下功能不可用：
- 插件市场浏览和下载
- 账号登录/注册
- 数据实时同步

## 可用功能

- 已安装插件正常使用
- 本地数据读写
- 快捷键唤起
- 悬浮窗显示

## 数据同步机制

每个插件的同步可独立控制：

```toml
# ~/.config/quick-tool/sync-control.toml
[[plugin_sync]]
tool_id = "markdown-note"
sync_enabled = true

[[plugin_sync]]
tool_id = "password-manager"
sync_enabled = false  # 禁用自动同步
```

退出离线模式时，暂存的数据变更自动同步：
- 工具配置变更
- 数据存储变更

## 启动方式

```bash
# 离线启动
cargo run --bin qt-client -- offline

# 调试日志模式
RUST_LOG=debug cargo run --bin qt-client -- offline
```
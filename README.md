# Quick Tool

跨平台 Rust 快捷工具，类似 Alfred/Raycast。

## 功能特性

- 🚀 **快捷键唤起**：Command+Space 快速启动，支持自定义快捷键
- 🔍 **关键字搜索**：快速过滤和查找工具
- 📦 **WASM 插件系统**：安全沙箱运行自定义工具，工具 UI 使用 Slint 定义
- 🔐 **端到端加密**：密码管理器在 WASM 内完整加密，宿主无法访问明文
- 🌐 **多服务端支持**：同时连接多个服务端，数据隔离，每个服务端插件独立
- 📱 **悬浮窗**：常驻显示小组件（时钟、系统监控等）
- 🔄 **离线模式**：支持离线启动调试，数据暂存退出后自动同步
- ⚙️ **插件同步控制**：每个插件的同步可独立开关

## 项目结构

```
quick-tool/
├── crates/
│   ├── qt-client/      # 客户端（Slint GUI）
│   ├── qt-runtime/     # WASM 运行时（Wasmtime）
│   ├── qt-sdk/         # 工具开发 SDK
│   ├── qt-core/        # 共享核心库
│   └── qt-server/      # 服务端（Axum + PostgreSQL）
├── tools/
│   ├── markdown-note/  # Markdown 记事本
│   └── password-manager/ # 密码管理器
├── proto/              # API 协议定义
└── docs/               # 文档
```

## 文档

- [调试启动命令](docs/debug-commands.md)
- [服务端部署](docs/server-deployment.md)
- [插件开发指南](docs/plugin-development.md)
- [离线模式说明](docs/offline-mode.md)

## 技术栈

| 模块 | 技术 |
|------|------|
| GUI | Slint |
| WASM 运行时 | Wasmtime |
| 服务端 | Axum + PostgreSQL |
| 客户端存储 | SQLite |
| 加密 | Argon2 + AES-256-GCM |
| 快捷键 | xdg-desktop-portal (Wayland) / XGrabKey (X11) |

## 系统依赖

### Linux

```bash
# Ubuntu/Debian
sudo apt-get install -y libfontconfig1-dev pkg-config

# Fedora
sudo dnf install fontconfig-devel pkg-config
```

### Wayland 支持

Wayland 下全局快捷键需要用户首次授权：
- 系统会弹出授权请求
- 在系统设置中授予 Quick Tool 快捷键权限

## License

MIT OR Apache-2.0
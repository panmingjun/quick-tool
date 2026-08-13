# Quick Tool - Claude AI 项目指南

## 项目概述

跨平台 Rust 快捷工具，类似 Alfred/Raycast。Command+Space 唤起，WASM 插件系统。

## 技术栈

- GUI: Slint
- WASM: Wasmtime
- 服务端: Axum + PostgreSQL
- 客户端存储: SQLite
- 快捷键: XGrabKey (X11) / xdg-desktop-portal (Wayland)

## 目录结构

```
crates/
├── qt-core/      # 共享类型、加密、错误处理
├── qt-client/    # Slint GUI、快捷键、离线模式
├── qt-runtime/   # WASM 运行时、沙箱
├── qt-sdk/       # 工具开发 SDK
├── qt-server/    # Axum REST API
tools/
├── markdown-note/      # Markdown 记事本
├── password-manager/   # 密码管理器 (WASM 内加密)
```

## 启动命令

```bash
cargo run --bin qt-client              # 正常启动
cargo run --bin qt-client -- --offline # 离线启动
cargo run --bin qt-server              # 服务端 (8080)
cargo run --bin markdown-note          # 工具独立调试
```

## 代码规范

- Rust edition 2021
- 中文注释
- 错误处理用 `qt_core::Result<T>`
- 日志用 tracing
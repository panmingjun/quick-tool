# Quick Tool 设计文档

> 本文档根据 README 与既有代码中的设计设想整理，作为系统的总体架构与模块设计蓝图。
> 实现状态标注约定：`已实现` / `骨架(部分)` / `规划(未实现)`。

## 1. 项目概述

### 1.1 定位

跨平台 Rust 快捷工具，交互形态对标 Alfred / Raycast：通过全局快捷键唤起启动器，按关键字快速检索并运行用户安装的工具。工具以 WASM 插件形式分发，在沙箱中运行，保证安全性与可扩展性。

### 1.2 设计原则

- **安全沙箱**：所有第三方工具运行在 WASM 沙箱内，按能力（Capability）白名单授权，宿主与插件严格隔离。
- **端到端加密**：敏感数据（如密码）在插件内完成加密，数据离开插件时仅以密文存在，宿主、服务端均无法获得明文。
- **离线优先**：客户端可在离线模式下运行，数据变更暂存本地，退出离线模式后按插件独立同步。
- **多服务端隔离**：客户端可同时连接多个服务端，服务端之间数据与插件完全隔离。
- **插件即应用**：插件的功能逻辑与 UI（Slint 定义）一起打包在 WASM 模块中，一次编译、跨平台分发。

### 1.3 功能特性清单

| 特性 | 说明 | 状态 |
|------|------|------|
| 快捷键唤起 | Command+Space 唤起启动器，快捷键可自定义 | 已实现 |
| 关键字搜索 | 快速过滤和查找工具 | 骨架 |
| WASM 插件系统 | 安全沙箱运行自定义工具，UI 用 Slint 定义 | 骨架 |
| 端到端加密 | 密码管理器在 WASM 内完整加密，宿主无法访问明文 | 已实现(核心) |
| 多服务端支持 | 同时连接多个服务端，数据隔离，每服务端插件独立 | 骨架 |
| 悬浮窗 | 常驻显示小组件（时钟、系统监控等） | 骨架 |
| 离线模式 | 支持离线启动调试，数据暂存退出后自动同步 | 已实现(核心) |
| 插件同步控制 | 每个插件的同步可独立开关 | 骨架 |

## 2. 系统架构

### 2.1 总体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      客户端 qt-client                        │
│  ┌───────────┐  ┌───────────┐  ┌─────────────────────────┐  │
│  │ Slint GUI │  │  窗口管理  │  │   快捷键 X11/Wayland     │  │
│  │ launcher/ │  │  launcher │  │   Command+Space 唤起     │  │
│  │ floating/ │  │  floating │  └─────────────────────────┘  │
│  │ market等  │  │  tool     │                                │
│  └─────┬─────┘  └─────┬─────┘                                │
│        └──────────────┼────────────────┐                     │
└───────────────────────┼────────────────┼─────────────────────┘
                        │                │
             ┌──────────▼──────────┐ ┌───▼─────────────────────┐
             │  qt-runtime (宿主)   │ │  本地存储 SQLite         │
             │  Wasmtime Engine     │ │  配置/暂存数据          │
             │  Sandbox 能力授权     │ │  ServerConfig / 离线缓存 │
             │  Loader / Instance   │ └─────────────────────────┘
             └──────────┬──────────┘
                        │
             ┌──────────▼──────────┐
             │  WASM 插件           │   ← 工具输出物，由 qt-sdk 开发
             │  工具逻辑 + Slint UI │      （markdown-note / password-manager）
             └─────────────────────┘
```

```
┌─────────────────────────── 服务端 qt-server (Axum) ───────────────────────────┐
│  /api/v1/auth    注册/登录/刷新/登出        (JWT)                              │
│  /api/v1/backup  备份 上传/下载/列表        (加密数据透传)                       │
│  /api/v1/tools   工具市场 列表/详情/下载/评分                                 │
└──────────────────────────────┬───────────────────────────────────────────────┘
                              │ sqlx
                       ┌──────▼──────┐
                       │ PostgreSQL  │   users / backups / tools
                       └─────────────┘
```

### 2.2 模块职责

| 模块 | 职责 | 依赖 |
|------|------|------|
| `qt-core` | 公共类型（ToolId/ServerId/UserId/ToolMetadata/Capability）、错误类型、加密原语（Argon2id + AES-256-GCM） | 无（被所有模块依赖） |
| `qt-client` | Slint GUI、窗口管理、快捷键（X11/Wayland）、配置管理、离线模式、服务端管理 | qt-core |
| `qt-runtime` | Wasmtime 引擎封装、沙箱隔离、模块加载、实例管理、能力实现 | qt-core、wasmtime |
| `qt-sdk` | 插件开发工具包：Tool trait、宿主 API（storage/http/clipboard/crypto）、Slint 组件 | qt-core |
| `qt-server` | Axum REST API：认证、数据备份、工具分发市场 | qt-core、sqlx |
| `tools/*` | 基于 qt-sdk 开发的插件示例/正式工具 | qt-sdk |

### 2.3 目录结构

```
crates/
├── qt-core/      # 共享类型、错误、加密
├── qt-client/    # Slint GUI、快捷键、离线模式、多服务端
├── qt-runtime/   # WASM 运行时、沙箱、能力实现
├── qt-sdk/       # 插件开发 SDK
└── qt-server/    # Axum REST API、数据库
tools/
├── markdown-note/      # Markdown 记事本
└── password-manager/   # 密码管理器（WASM 内加密）
proto/
└── openapi.yaml        # 服务端 REST API 协议定义
docs/                   # 本文档及运维/开发文档
```

## 3. 客户端设计

### 3.1 应用整体状态

应用由 `AppState` 持有全局状态：离线状态（`OfflineState`）、调试插件 ID、窗口可见性。
`AppState` 以 `Arc<Mutex<AppState>>` 在快捷键线程与 UI 线程间共享，UI 操作通过 `slint::invoke_from_event_loop` 回到主线程执行。

### 3.2 窗口管理

`window` 模块定义窗口类型 `WindowType`：

- `Launcher`：主启动器窗口，默认隐藏，由快捷键/搜索唤起。
- `Floating`：悬浮窗，常驻显示小组件（时钟、系统监控），可置顶、调节透明度和位置。
- `Tool`：插件工具窗口，承载 WASM 插件渲染的 Slint UI。

`window/layer` 预留系统层（Layer）窗口支持，用于悬浮窗的无边框置顶显示。

### 3.3 快捷键系统

- `config/hotkey`：定义 `Hotkey`（`modifiers` + `key`）与 `HotkeyConfig`，默认 `Command+Space`（唤起启动器）、`Command+Shift+Space`（唤起悬浮窗）；`Super` 跨平台映射（Windows 键 / macOS Command / Linux Super）。
- `hotkey/platform`：抽象 `HotkeyManager` Trait（`register` / `unregister` / `listen`），监听线程通过 `mpsc::Receiver<HotkeyEvent>` 推送事件。
- 平台实现：
  - X11：基于 `x11rb` + `XGrabKey` 捕获全局按键。
  - Wayland：基于 `xdg-desktop-portal`（通过 `ashpd`/`smithay-client-toolkit`）实现全局快捷键。
- 平台选择：启动时检测 `WAYLAND_DISPLAY` 或 `XDG_SESSION_TYPE` 决定使用 Wayland 还是 X11 实现。

当前监听线程只处理唤起切换逻辑（显示/隐藏），自定义快捷键与多快捷键分发为后续演进方向。

### 3.4 配置管理

配置以 TOML 存储在 XDG 标准目录（`ProjectDirs::from("io","QuickTool","quick-tool")`）：

- `hotkey.toml`：快捷键配置。
- `server.toml`：服务端配置列表 `ServerConfigList { servers: Vec<ServerConfig> }`，默认提供一个官方服务端 `https://api.quicktool.io`。
- `sync-control.toml`：插件同步控制（见 §6.3）。

### 3.5 多服务端支持

- 客户端维护 `ServerConnection` 列表：`ServerConfig`（id/name/address/is_default）+ `auth_token` + `ConnectionStatus`。
- 每个服务端拥有独立的插件集与数据存储，切换服务端即切换"插件工作区"。
- 服务端管理界面支持添加/编辑/删除/默认服务端。

### 3.6 关键字搜索

`search` 模块维护搜索状态（query/focused）；`launcher` 根据关键字对已安装工具与命令进行过滤排序。当前为骨架实现，预计匹配字段包含工具 `name` / `keywords` / `description`。

### 3.7 悬浮窗

`floating` 模块定义悬浮窗位置、大小、透明度、置顶属性。用于常驻显示小组件（时钟、系统监控等），小组件可设计为 miniapp 型插件的渲染载体。

## 4. WASM 插件系统

### 4.1 运行时架构

`qt-runtime` 基于 Wasmtime 封装：

- **`engine::WasmEngine`**：封装 `wasmtime::Engine` 与 `Linker<HostState>`。引擎启用 `wasm_backtrace_details`（更友好的错误回溯）、`bulk_memory`（性能）、`consume_fuel`（燃料计量，用于 CPU 用量限制）。
- **`loader::ModuleLoader`**：从文件或内存加载 WASM 二进制，包装为 `LoadedModule`。
- **`instance::InstanceManager`**：管理 `ToolInstance`（id + module + sandbox）的创建/查询/销毁。
- **`sandbox::Sandbox`**：基于 `SandboxConfig` 做能力与网络白名单校验；宿主 `HostState` 携带能力授权列表，供 Linker 中的宿主函数回溯授权状态。

### 4.2 沙箱与能力授权

`Capability` 枚举（qt-core）：

| 能力 | 说明 |
|------|------|
| `Storage` | 键值数据读写（经宿主持久化，范围限定在插件数据目录） |
| `Network` | HTTP 请求（受域名白名单限制） |
| `Clipboard` | 读写系统剪贴板 |
| `Screen` | 屏幕信息与截图 |
| `SystemInfo` | 系统信息查询 |

能力经静态声明（`ToolMetadata.capabilities`）与运行时双重校验：
- 声明期：安装/加载时记录插件声明的能力集合，构建 `SandboxConfig`。
- 运行时：宿主函数被调用时经 `Sandbox::check_capability` / `check_network_access` 校验，未授权返回 `CapabilityDenied`。

能力细化控制（qt-runtime/capability）：剪贴板区分读/写，屏幕区分信息/截图，文件系统限定允许路径，网络限定允许域名。

### 4.3 插件生命周期

插件实现 `qt_sdk::Tool` Trait，宿主按事件驱动生命周期：

```
加载模块 ──► 创建 Sandbox(能力校验) ──► 创建 ToolInstance ──► 激活 on_activate
                                                                │
                                                         触发全局事件
                                                                │
                                                         ┌──────▼──────┐
                                                         │ on_suspend  │ ◄── 隐藏/切换/失去焦点
                                                         └─────────────┘
```

- `on_activate`：插件被唤起时执行初始化与 UI 展示。
- `on_suspend`：插件被隐藏/切换时挂起，释放资源。
- 实例销毁：`InstanceManager::destroy_instance` 释放 WASM 实例与沙箱。

### 4.4 资源限制（规划）

基于已启用的 `consume_fuel` 对每个实例设置燃料配额，超限即中断，防止插件无限循环消耗 CPU；内存限制通过 Wasmtime 实例内存上限配置；文件系统与网络仅经宿主能力出口访问。

### 4.5 SDK 设计（qt-sdk）

`qt-sdk` 是插件开发工具包，宿主侧调用（非 WASM 内部 syscall）：

```
qt-sdk
├── Tool Trait              # metadata / on_activate / on_suspend
└── api
    ├── storage    StorageApi    # get / set / delete / keys（键值存储）
    ├── http       HttpApi       # get / post（受网络白名单约束）
    ├── clipboard  ClipboardApi  # get_text / set_text / get_image
    └── crypto     CryptoApi     # derive_key / encrypt / decrypt / generate_salt
```

- **StorageApi**：插件数据的持久化入口，写入由宿主隔离的插件数据目录，可同时作为同步单位（见 §6）。
- **HttpApi**：插件网络出口，请求经宿主代理发出并受 `SandboxConfig.network_whitelist` 约束。
- **CryptoApi**：加密 API 以宿主提供的密码学函数形式暴露。注意：密码管理器这类要求端到端安全的插件，选择在 WASM 内用 `qt_core::crypto` 直接计算，宿主仅做计算卸载，不接触明文密钥。

### 4.6 插件 UI：Slint 定义

插件 UI 使用 Slint 声明式定义，随 WASM 模块一起分发。宿主把插件渲染进 `Tool` 窗口，UI 数据与插件逻辑通过宿主桥接（callback/property）联动。`qt-sdk/component/common.slint` 提供通用组件（按钮、输入框、列表等）供插件复用，保证观感一致。

### 4.7 端到端加密模型

以密码管理器为例：

```
插件 (WASM 沙箱)                             宿主 / 服务端
────────────────────                      ────────────────
主密码 ──► Argon2id ──► master_key(仅存内存)
                              │
   明文密码 ──► AES-256-GCM ──► ciphertext + nonce
                                        │
                             密文经 Storage/备份通道持久化   ← 只能看到密文
```

- 密钥派生：`Argon2id` + 随机 salt（`qt_core::crypto::derive_key`）。
- 加密：`AES-256-GCM`，12 字节随机 nonce，密文与 nonce 一并存储（`qt_core::crypto::encrypt/decrypt`）。
- 明文与主密钥只存在于 WASM 实例内存；宿主、服务端、其它插件均无法解密。
- 加密数据以插件自身数据为单位存储与备份，宿主按不透明 blob 透传。

## 5. 客户端本地存储

- **SQLite**：客户端主数据存储（插件数据、元数据、配置索引），规划数据库结构（表结构待明确）。
- **TOML 配置**：快捷键、服务端列表、同步控制等非频繁变更配置。
- **离线暂存**：离线期间的变更以内存/本地形式暂存，退出时同步（见 §6）。

## 6. 离线模式与数据同步

### 6.1 离线状态机

`OfflineState`（qt-client/config/offline）维护离线状态与待同步数据：

```
enter_offline ──► is_offline = true   记录变更到 pending_sync
exit_offline  ──► is_offline = false  取出 pending_sync，逐条同步到服务端
```

`--offline` 启动参数进入离线模式（对应 CLI `--offline`，注意 docs/offline-mode.md 中的 `offline` 子命令旧写法）。

### 6.2 暂存模型

`PendingSyncData` 包含两类变更记录：

- `ToolConfigChange`：插件配置变更（tool_id + server_id + config + changed_at）。
- `DataChange`：插件数据变更（id + server_id + data_type + encrypted_blob + changed_at）。

暂存数据按 `server_id` 归属对应服务端，退出离线模式后向各服务端按序推送，成功后清空。

### 6.3 插件同步控制

`sync-control.toml` 按插件独立控制自动同步：

```toml
[[plugin_sync]]
tool_id = "markdown-note"
sync_enabled = true

[[plugin_sync]]
tool_id = "password-manager"
sync_enabled = false  # 禁用自动同步（高敏感数据可仅本地）
```

- `sync_enabled = false` 的插件数据不参与服务端备份/同步。
- 同步控制、最后的 `last_sync_at` 与离线暂存逻辑配合，实现"仅同步允许同步的变更"。

### 6.4 离线功能边界

可用：已安装插件正常运行、本地数据读写、快捷键唤起、悬浮窗显示。
不可用：插件市场浏览下载、账号登录注册、数据实时同步。

## 7. 服务端设计

### 7.1 REST API（Axum）

协议定义见 `proto/openapi.yaml`，路由挂载于 `/api/v1`：

| 模块 | 路由 | 说明 |
|------|------|------|
| Auth | `POST /auth/register` | 用户注册 |
| Auth | `POST /auth/login` | 用户登录，返回 access token + refresh token |
| Auth | `POST /auth/refresh` | 刷新 token |
| Auth | `POST /auth/logout` | 登出 |
| Backup | `GET /backup` | 备份列表 |
| Backup | `POST /backup/upload` | 上传备份（data_type + encrypted_data，密文透传） |
| Backup | `GET /backup/download/{id}` | 下载备份 |
| Tools | `GET /tools` | 工具列表（含下载数、评分） |
| Tools | `GET /tools/{id}` | 工具详情 |
| Tools | `GET /tools/{id}/download` | 下载工具 WASM |
| Tools | `POST /tools/{id}/rate` | 工具评分 |

### 7.2 认证与令牌（规划）

使用 JWT：`ServerConfig.jwt_secret` 签名；登录返回 access/refresh 双 token；API 认证中间件校验 `Authorization: Bearer`，`GET /tools` 等公开接口可匿名访问。当前 handler 为 TODO 骨架，注册将使用 Argon2 对密码哈希存储（与客户端派生算法一致但用途不同：服务端存储密码哈希，插件密钥派生由用户掌握）。

### 7.3 数据库设计（PostgreSQL）

| 表 | 关键字段 | 说明 |
|----|----------|------|
| `users` | id (UUID PK)、username (unique)、email (unique)、password_hash、created_at、updated_at | 账号 |
| `backups` | id、user_id (FK→users)、data_type、encrypted_data (BYTEA)、created_at | 加密备份，服务端只存密文 |
| `tools` | id、name、description、version、author、wasm_data (BYTEA)、download_count、rating、created_at | 分发市场 |

表由 `db/schema.rs` 的 `create_tables` 在启动时 `CREATE TABLE IF NOT EXISTS` 初始化。

### 7.4 服务层

- `UserService`：注册（密码哈希）、登录校验。
- `BackupService`：备份上传/下载，服务端不感知数据内容（密文）。
- `ToolService`：工具列表、WASM 数据获取。

## 8. 端到端数据流

插件数据备份场景：

```
插件写入 ──► StorageApi ──► SQLite(本地, 密文)
      ▼ 同步开启 且 非离线
      ▼ per-plugin sync control
服务端 /backup/upload (encrypted_data) ──► backups(users 归属)
      ▼ 其他设备下载 /backup/download ──► 本地 SQLite ──► 插件读取解密
```

插件分发场景：

```
qt-server /tools/{id}/download(WASM) ──► qt-runtime loader ──► Sandbox 校验 ──► ToolInstance ──► Tool 窗口渲染
```

## 9. 部署与运维

- **Docker Compose**：`postgres:16-alpine` + 服务端镜像（`Dockerfile.server`），端口 5432/8080，服务端等待数据库健康后再启动。
- **环境变量**：`DATABASE_URL`、`JWT_SECRET`、`PORT`、`RUST_LOG`（详见 `docs/server-deployment.md`）。
- **本地数据**：客户端配置与数据位于 XDG 目录，`--offline` 可无服务端运行。

## 10. 安全模型

| 威胁 | 缓解 |
|------|------|
| 恶意插件读取宿主敏感数据 | WASM 沙箱 + 能力白名单，仅经宿主能力出口 |
| 恶意插件消耗资源 | 燃料计量（consume_fuel）+ 实例内存上限 |
| 插件越权访问网络 | 网络域名白名单 |
| 备份数据泄露 | 服务端仅存密文（端到端加密） |
| 非授权访问 API | JWT 认证 + refresh token |

## 11. 开发与调试

- 独立调试：`cargo run --bin markdown-note` / `cargo run --bin password-manager` 可在不启动宿主的场景跑通工具逻辑。
- 编译 WASM：`rustup target add wasm32-unknown-unknown`，`cargo build --package <tool> --target wasm32-unknown-unknown`。
- 详细命令见 `docs/debug-commands.md`。

## 12. 路线图

按当前实现阶段（骨架 / TODO 较多）规划：

1. **插件机制核心闭环**：宿主加载 WASM → 实例化 → 调用导出函数 → 读取内存（`qt-runtime/plugin` + `hello-plugin` 已验证）。
2. **先完成后端服务**：auth（注册/登录/JWT）、backup、tools 的 handler 与 sqlx 查询落地。
3. **运行时接通**：Wasmtime Linker 宿主函数完整接入 SDK API（storage/http/clipboard/crypto），实现能力出口。
4. **插件市场闭环**：客户端 market UI 对接 /tools，下载 → 校验 → 加载 → 运行 → 评分。
5. **离线同步完整实现**：退出离线模式的自动同步逻辑（目前 app.rs 中为 TODO）。
6. **悬浮窗小组件**：Widget 型插件 + layer 窗口支持。
7. **SQLite 化**：客户端本地存储从内存/TOML 迁移到 SQLite 完整 schema。
8. **打包发布**：桌面安装包、插件签名与校验、服务端 HTTPS 上线。

## 附录 A：实现状态速览

| 组件 | 状态 | 主要 TODO |
|------|------|-----------|
| qt-core 类型/加密/错误 | 已实现 | — |
| qt-client 快捷键 | 已实现 | 自定义快捷键 |
| qt-client 离线状态 | 已实现 | 退出同步逻辑 |
| qt-client UI | 骨架 | Slint 完整界面 |
| qt-runtime 插件执行(plugin) | 已实现 | 宿主函数与能力出口 |
| qt-runtime 引擎/沙箱/实例 | 骨架 | Linker 宿主函数 |
| qt-sdk API | 骨架 | 宿主桥接实现 |
| tools/markdown-note、password-manager | 示例级 | 完整功能 |
| tools/hello-plugin | 已实现 | 验证用最小 WASM 插件 |
| qt-server auth/backup/tools | 骨架 | handler + sqlx 查询 |

## 附录 B：参考文档

- [插件接入规范](plugin-spec.md)
- [插件开发指南](plugin-development.md)
- [服务端部署](server-deployment.md)
- [离线模式说明](offline-mode.md)
- [调试启动命令](debug-commands.md)
- [服务端 API 协议](../../proto/openapi.yaml)
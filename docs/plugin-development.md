# 插件开发指南

## 概述

Quick Tool 插件使用 Rust 开发，编译为 WASM 模块运行。插件 UI 使用 Slint 定义。

- 插件逻辑与 UI 捆绑在同一个 WASM 模块中，随插件分发，一次编译跨平台运行。
- 插件运行在 Wasmtime 沙箱中，只能通过宿主暴露的 API 与外部世界交互，并按声明的能力白名单授权。
- 敏感数据（如密码）遵循端到端加密模型：明文只存在于插件自身内存，宿主与服务端只能看到密文。

## 开发环境

```bash
# 安装 WASM 编译目标
rustup target add wasm32-unknown-unknown

# 构建项目
cargo build --workspace

# 编译单个工具为 WASM
cargo build --package markdown-note --target wasm32-unknown-unknown --release

# 独立调试（不依赖宿主）
cargo run --bin markdown-note
cargo run --bin password-manager
```

## 项目结构

插件位于 `tools/` 目录，每个插件是一个独立 crate：

```
tools/markdown-note/
├── Cargo.toml        # 依赖 qt-sdk、qt-core；设置 crate-type = ["cdylib"]
└── src/
    ├── lib.rs        # 插件实现（实现 qt_sdk::Tool）
    └── main.rs       # 独立调试入口（可选，方便脱离宿主调试）
```

`Cargo.toml` 要点：

```toml
[package]
name = "markdown-note"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]          # 产出 WASM 模块

[dependencies]
qt-sdk = { path = "../../crates/qt-sdk" }
qt-core = { path = "../../crates/qt-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 独立调试可选：tracing / tracing-subscriber
```

## SDK 使用

插件必须实现 `qt_sdk::Tool` Trait：

```rust
pub trait Tool: Send + Sync {
    // 工具元信息（含声明的能力列表）
    fn metadata(&self) -> ToolMetadata;
    // 插件被唤起时调用
    fn on_activate(&mut self);
    // 插件被挂起时调用
    fn on_suspend(&mut self);
}
```

最小插件骨架：

```rust
use qt_sdk::Tool;
use qt_core::{ToolMetadata, ToolId, Capability};

pub struct MyTool {
    id: ToolId,
}

impl MyTool {
    pub fn new() -> Self {
        Self { id: ToolId::new() }
    }
}

impl Tool for MyTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: self.id.clone(),
            name: "我的工具".into(),
            description: "工具描述".into(),
            version: "1.0.0".into(),
            author: "作者".into(),
            keywords: vec!["keyword".into()],
            capabilities: vec![Capability::Storage],   // 声明所需能力
        }
    }

    fn on_activate(&mut self) {
        // 初始化与展示
    }

    fn on_suspend(&mut self) {
        // 挂起与清理
    }
}
```

## 元信息与能力声明

`ToolMetadata` 字段：

| 字段 | 说明 |
|------|------|
| `id` | 插件唯一标识（`ToolId`，UUID） |
| `name` | 显示名称 |
| `description` | 功能描述 |
| `version` | 语义化版本 |
| `author` | 作者 |
| `keywords` | 搜索关键字（供启动器检索） |
| `capabilities` | 声明的能力列表（见下） |

`Capability` 能力：

| 能力 | 权限 |
|------|------|
| `Storage` | 插件键值数据存储（范围限定在插件数据目录） |
| `Network` | HTTP 请求（受域名白名单约束） |
| `Clipboard` | 读写剪贴板 |
| `Screen` | 屏幕信息与截图 |
| `SystemInfo` | 系统信息 |

能力在安装时校验并构建沙箱配置，运行时请求未经授权的能力会返回 `CapabilityDenied` 错误。

## Slint UI 定义

插件 UI 使用 Slint 编写。`qt-sdk` 提供通用组件（`qt-sdk/src/component/common.slint`）供复用。

```slint
// 插件 UI 示例
import { Button, LineEdit } from "../../crates/qt-sdk/src/component/common.slint";

export component NoteUI inherits Window {
    in property <string> content;
    callback save(string);
    // ...
}
```

宿主将插件组件渲染进工具窗口，通过 property / callback 与插件逻辑桥接。

## 宿主 API

通过 `qt_sdk::api` 暴露宿主能力：

### 数据存储 StorageApi

```rust
trait StorageApi {
    fn get(&self, key: &str) -> qt_core::Result<Option<String>>;
    fn set(&mut self, key: &str, value: &str) -> qt_core::Result<()>;
    fn delete(&mut self, key: &str) -> qt_core::Result<()>;
    fn keys(&self) -> qt_core::Result<Vec<String>>;
}
```

- 数据按插件隔离，写入后由宿主持久化。
- 存储数据同时是备份/离线同步的单位，遵循 §8 的同步控制。

**插件侧调用方式（已实现）**：WIT `qt:plugin.storage` import，值均为 UTF-8 字符串。
每个插件一个独立 SQLite 库（`data/plugins/<plugin_id>/data.sqlite`），写入受配额限制
（默认 10 MiB/插件，超限返回 `kv-error.quota-exceeded`）：

```rust
use qt_sdk::bindings::qt::plugin::storage;

fn dispatch_action(action: String) -> bool {
    match action.as_str() {
        "save" => {
            // 超配额时返回 Err(KvError::QuotaExceeded(quota))
            let _ = storage::kv_set("note", "hello");
            true
        }
        "load" => {
            let _value: Option<String> = storage::kv_get("note");
            true
        }
        _ => false,
    }
}
```

可用函数：`kv-get` / `kv-set` / `kv-delete` / `kv-keys` / `used-bytes`。

### HTTP 请求 HttpApi

```rust
trait HttpApi {
    fn get(&self, url: &str) -> qt_core::Result<Vec<u8>>;
    fn post(&self, url: &str, body: &[u8]) -> qt_core::Result<Vec<u8>>;
}
```

请求经宿主代理发出，目标域名必须命中沙箱网络白名单。

### 剪贴板 ClipboardApi

```rust
trait ClipboardApi {
    fn get_text(&self) -> qt_core::Result<Option<String>>;
    fn set_text(&self, text: &str) -> qt_core::Result<()>;
    fn get_image(&self) -> qt_core::Result<Option<Vec<u8>>>;
}
```

### 加密 CryptoApi

```rust
trait CryptoApi {
    fn derive_key(&self, password: &str, salt: &[u8]) -> qt_core::Result<[u8; 32]>;
    fn encrypt(&self, key: &[u8; 32], plaintext: &[u8]) -> qt_core::Result<(Vec<u8>, [u8; 12])>;
    fn decrypt(&self, key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> qt_core::Result<Vec<u8>>;
    fn generate_salt(&self, size: usize) -> Vec<u8>;
}
```

## 加密 API 与端到端加密

对需要端到端安全的插件（如密码管理器），在 WASM 内直接使用 `qt_core::crypto`：

```rust
// 设置主密码：Argon2id 派生密钥（盐随机生成）
let salt = qt_core::crypto::generate_salt(16);
let master_key = qt_core::crypto::derive_key(password, &salt)?;

// 加密：AES-256-GCM，返回 (ciphertext, nonce)
let (ciphertext, nonce) = qt_core::crypto::encrypt(&master_key, plaintext.as_bytes())?;

// 解密
let plaintext = qt_core::crypto::decrypt(&master_key, &nonce, &ciphertext)?;
```

要点：

- 密钥派生 `Argon2id`（`DEFAULT_M_COST/T_COST/P_COST`，输出 32 字节）。
- 加密 `AES-256-GCM`，12 字节随机 nonce。
- 密文与 nonce 一起存储（如 `PasswordEntry.encrypted_password` + `nonce`），宿主按不透明 blob 持久化与同步。
- 主密钥只保存在插件实例内存中，宿主、服务端、其它插件均无法解密。

参考实现：`tools/password-manager/src/lib.rs`。

## 能力授权（宿主侧）

能力授权发生在插件加载时，由 `qt-runtime` 根据插件声明的 `capabilities` 构建 `SandboxConfig`：

- 数据路径限制：插件仅可访问自己的数据目录。
- 网络白名单：`network_whitelist`，为空表示不限制，否则按前缀匹配。
- 细粒度控制：剪贴板读/写、屏幕信息/截图可分别授权。

```rust
let sandbox = Sandbox::new(SandboxConfig {
    capabilities: vec![Capability::Storage],
    data_path: Some(plugin_data_dir),
    network_whitelist: vec!["https://api.example.com".into()],
});
```

非授权能力调用返回 `qt_core::Error::CapabilityDenied`。

## 发布流程

1. 完成开发并在 `tools/` 下独立调试（`cargo run --bin <tool>`）。
2. 编译 WASM：

```bash
rustup target add wasm32-unknown-unknown
cargo build --package <tool> --target wasm32-unknown-unknown --release
```

产物位于 `target/wasm32-unknown-unknown/release/<tool>.wasm`。

3. 使用服务端 API 发布（当前为规划中的流程，服务端正在实现）：

```bash
# 1) 登录获取 token
curl -X POST https://api.quicktool.io/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"...","password":"..."}'

# 2) 上传工具 WASM（开发中）
curl -X POST https://api.quicktool.io/api/v1/tools \
  -H 'Authorization: Bearer <token>' \
  -F 'wasm=@target/wasm32-unknown-unknown/release/<tool>.wasm' \
  -F 'name=<tool>' -F 'version=1.0.0' -F 'description=...'
```

4. 审核发布后，用户在客户端市场下载安装。
5. 安装校验（规划）：对 WASM 进行签名校验，防止分发链路篡改。

## 调试与测试

- 独立调试入口 `main.rs`：直接实例化插件并调用 `on_activate/on_suspend`，手动给宿主 API 传桩，脱离 GUI 验证逻辑。
- 日志：使用 `tracing`，宿主侧 `RUST_LOG=debug` 查看运行时输出。
- 建议为能力需求单一的小工具独立编写 `#[cfg(test)]` 单元测试。

## 参考

- [总体设计文档](design.md)
- [调试启动命令](debug-commands.md)
- [离线模式说明](offline-mode.md)
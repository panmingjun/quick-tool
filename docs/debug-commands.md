# 调试启动命令

## 客户端启动

```bash
# 正常启动客户端
cargo run --bin qt-client

# 离线启动客户端（不连接服务端）
cargo run --bin qt-client -- --offline

# 带调试日志启动
RUST_LOG=debug cargo run --bin qt-client
RUST_LOG=debug cargo run --bin qt-client -- --offline
```

## 工具独立调试

工具可以在 `tools` 目录下独立运行调试：

```bash
# 调试 Markdown 记事本
cargo run --bin markdown-note

# 调试密码管理器（测试加密功能）
cargo run --bin password-manager
```

## 服务端启动

```bash
# 启动服务端
cargo run --bin qt-server

# 带调试日志启动
RUST_LOG=debug cargo run --bin qt-server
```

## Docker 部署

```bash
# 一键启动服务端（PostgreSQL + 服务端）
docker-compose up -d

# 查看日志
docker-compose logs -f server

# 停止服务
docker-compose down
```

## 编译 WASM

```bash
# 编译所有工具
cargo build --workspace

# 编译测试插件（核心机制演示需要，qt-client 默认从 release 目录加载）
rustup target add wasm32-unknown-unknown
cargo build -p hello-plugin --target wasm32-unknown-unknown --release

# 编译单个工具为 WASM（需要 wasm32-unknown-unknown target）
cargo build --package markdown-note --target wasm32-unknown-unknown
cargo build --package password-manager --target wasm32-unknown-unknown
```

## 插件机制演示

1. 先编译测试插件（生成 `target/wasm32-unknown-unknown/release/hello_plugin.wasm`）：

```bash
cargo build -p hello-plugin --target wasm32-unknown-unknown --release
```

2. 启动客户端（会自动扫描 `plugins/` 下本地插件）：

```bash
cargo run --bin qt-client
```

3. 在主窗口的插件列表中点击「Hello 测试插件」，进入插件窗口。
4. 插件窗口渲染插件 `get-ui` 返回的稳定模板（含按钮）；点击按钮后宿主转发 `dispatch-action` 给插件，插件更新内部状态，宿主轮询 `get-state` 拿到数据快照并 `set_property` 增量更新，文本显示 `Hello, World!`。点击「返回列表」可回到插件列表。

插件接入与 ABI 约定见 [插件接入规范](plugin-spec.md)。
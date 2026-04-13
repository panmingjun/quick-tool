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

# 编译单个工具为 WASM（需要 wasm32-unknown-unknown target）
rustup target add wasm32-unknown-unknown
cargo build --package markdown-note --target wasm32-unknown-unknown
cargo build --package password-manager --target wasm32-unknown-unknown
```
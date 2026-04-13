# 服务端部署

## Docker Compose 快速部署

### 1. 准备配置

```bash
# 复制环境变量配置
cp .env.example .env

# 编辑配置（修改 JWT_SECRET）
vim .env
```

### 2. 启动服务

```bash
# 构建并启动所有服务
docker-compose up -d

# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f server
```

### 3. 数据库初始化

服务启动后会自动创建数据库表。如需手动初始化：

```bash
# 连接到数据库
docker-compose exec postgres psql -U quicktool -d quicktool

# 查看表结构
\dt
```

### 4. 停止服务

```bash
# 停止所有服务
docker-compose down

# 停止并删除数据
docker-compose down -v
```

## 手动部署

### 环境要求

- Rust 1.70+
- PostgreSQL 14+

### 步骤

1. 安装 PostgreSQL 并创建数据库：

```sql
CREATE USER quicktool WITH PASSWORD 'your_password';
CREATE DATABASE quicktool OWNER quicktool;
```

2. 设置环境变量：

```bash
export DATABASE_URL=postgres://quicktool:your_password@localhost:5432/quicktool
export JWT_SECRET=your_jwt_secret
export PORT=8080
```

3. 编译并运行：

```bash
cargo build --release -p qt-server
./target/release/qt-server
```

## 配置说明

| 变量 | 说明 | 默认值 |
|------|------|--------|
| DATABASE_URL | PostgreSQL 连接字符串 | 必填 |
| JWT_SECRET | JWT 签名密钥 | 必填 |
| PORT | 服务端口 | 8080 |
| RUST_LOG | 日志级别 | info |
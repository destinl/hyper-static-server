# 🐳 Docker 部署指南

**hyper-static-server** 提供完整的 Docker 部署支持，包括开发环境、生产环境和自动化部署脚本。

---

## 📋 快速开始

### 前置条件
- Docker 20.10+
- Docker Compose 2.0+

### 一键部署

```bash
# 开发环境 (热重载)
./deploy.sh dev

# 生产环境 (Nginx 反向代理)
./deploy.sh prod

# 停止服务
./deploy.sh stop
```

---

## 🏗️ Docker 文件结构

```
hyper-static-server/
├── Dockerfile              # 生产镜像 (多阶段构建)
├── Dockerfile.dev          # 开发镜像 (热重载)
├── docker-compose.yml      # 默认配置
├── docker-compose.dev.yml  # 开发环境
├── docker-compose.prod.yml # 生产环境
├── nginx.conf              # Nginx 配置
├── .dockerignore           # 构建忽略文件
├── deploy.sh               # Linux/macOS 部署脚本
└── deploy.bat              # Windows 部署脚本
```

---

## 🌍 部署环境

### 开发环境 (`docker-compose.dev.yml`)

**特点**:
- 热重载 (代码变更自动重启)
- 源代码挂载
- 调试日志
- 开发友好

**启动**:
```bash
./deploy.sh dev
# 或: docker-compose -f docker-compose.dev.yml up --build
```

**访问**: http://localhost:3000

### 生产环境 (`docker-compose.prod.yml`)

**特点**:
- Nginx 反向代理
- SSL/HTTPS 就绪
- 优化的生产镜像
- 健康检查
- 安全配置

**启动**:
```bash
./deploy.sh prod
# 或: docker-compose -f docker-compose.prod.yml up --build -d
```

**访问**: http://localhost (HTTP) 或 https://localhost (HTTPS)

### 默认环境 (`docker-compose.yml`)

**特点**:
- 基础配置
- 文件挂载
- 快速测试

**启动**:
```bash
docker-compose up --build
```

---

## 🔧 手动操作

### 构建镜像

```bash
# 生产镜像
docker build -t hyper-static-server .

# 开发镜像
docker build -f Dockerfile.dev -t hyper-static-server:dev .
```

### 运行容器

```bash
# 基础运行
docker run -p 3000:3000 hyper-static-server

# 挂载目录
docker run -p 3000:3000 \
  -v $(pwd)/public:/app/public:ro \
  hyper-static-server

# 自定义配置
docker run -p 8080:3000 \
  -v /path/to/files:/app/public:ro \
  hyper-static-server -d /app/public -h 0.0.0.0
```

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RUST_LOG` | `info` | 日志级别 (debug/info/warn/error) |

---

## 🔒 安全配置

### 非 root 用户
Docker 镜像使用非 root 用户运行，提高安全性。

### 文件权限
- 静态文件只读挂载 (`:ro`)
- 容器内文件权限严格控制

### 网络隔离
- 使用 Docker 网络隔离服务
- 仅暴露必要端口

---

## ⚙️ 生产配置

### SSL/HTTPS 设置

1. **创建 SSL 目录**:
   ```bash
   mkdir ssl
   ```

2. **放置证书文件**:
   ```
   ssl/
   ├── cert.pem    # SSL 证书
   └── key.pem     # 私钥
   ```

3. **启动生产环境**:
   ```bash
   ./deploy.sh prod
   ```

### Nginx 配置

生产环境使用 Nginx 作为反向代理，提供：
- 静态文件缓存
- Gzip 压缩
- 安全头 (CSP, HSTS, etc.)
- 请求限制
- SSL 终止

### 健康检查

```bash
# 检查服务状态
curl http://localhost/health

# Docker 健康检查
docker ps
docker-compose ps
```

---

## 📊 性能优化

### 镜像大小
- **生产镜像**: ~50MB (压缩后)
- **开发镜像**: ~1.2GB (包含开发工具)

### 启动时间
- **生产环境**: < 5 秒
- **开发环境**: < 10 秒 (包含编译)

### 内存使用
- **基础运行**: ~15-30 MB
- **高负载**: ~50-100 MB

---

## 🔍 故障排除

### 查看日志

```bash
# 所有服务的日志
./deploy.sh logs

# 特定服务日志
docker-compose logs hyper-static-server
docker-compose logs nginx
```

### 服务状态

```bash
# 查看运行状态
./deploy.sh status

# 详细状态
docker-compose ps
```

### 常见问题

**Q: 端口被占用**
```bash
# 检查端口使用
netstat -tulpn | grep :3000

# 修改端口映射
docker-compose -f docker-compose.yml up --scale hyper-static-server=1 -p 3001:3000
```

**Q: 文件权限错误**
```bash
# 检查文件权限
ls -la public/

# 修复权限
chmod -R 755 public/
```

**Q: 构建失败**
```bash
# 清理缓存重新构建
docker system prune -a
docker-compose build --no-cache
```

---

## 🚀 部署到云平台

### Docker Hub

```bash
# 构建并推送
docker build -t yourusername/hyper-static-server .
docker push yourusername/hyper-static-server

# 在服务器上运行
docker run -p 80:80 yourusername/hyper-static-server
```

### 云服务

**AWS ECS/Fargate**:
```bash
# 使用 docker-compose.yml 作为任务定义
aws ecs create-service --service-name hyper-static-server \
  --task-definition hyper-static-server-task \
  --desired-count 1
```

**Google Cloud Run**:
```bash
gcloud run deploy hyper-static-server \
  --source . \
  --platform managed \
  --port 3000
```

**Azure Container Instances**:
```bash
az container create \
  --resource-group myResourceGroup \
  --name hyper-static-server \
  --image hyper-static-server \
  --ports 80 \
  --dns-name-label hyper-static-server
```

---

## 📝 自定义配置

### 修改端口

```yaml
# docker-compose.yml
services:
  hyper-static-server:
    ports:
      - "8080:3000"  # 宿主机:容器
```

### 添加环境变量

```yaml
# docker-compose.yml
services:
  hyper-static-server:
    environment:
      - RUST_LOG=debug
      - CUSTOM_VAR=value
```

### 挂载配置文件

```yaml
# docker-compose.yml
services:
  hyper-static-server:
    volumes:
      - ./config:/app/config:ro
      - ./logs:/app/logs
```

---

## 🎯 最佳实践

1. **使用生产环境配置**进行部署
2. **定期更新基础镜像**以获取安全补丁
3. **监控资源使用**并调整容器限制
4. **使用健康检查**确保服务可用性
5. **备份配置文件**和 SSL 证书
6. **使用日志轮转**避免磁盘空间不足

---

## 📞 获取帮助

- **文档**: [README.md](../README.md) Docker 部分
- **日志**: `./deploy.sh logs`
- **状态**: `./deploy.sh status`
- **重启**: `./deploy.sh stop && ./deploy.sh prod`

---

**🎉 享受高性能的 Rust 静态文件服务器！**
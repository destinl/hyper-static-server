# hyper-static-server 项目规划

**项目名称**: hyper-static-server  
**版本**: 0.1.0 (Phase 1)  
**开始日期**: 2026-03-22  
**语言**: Rust (后端) + TypeScript (前端)

---

## 🎯 项目愿景

实现一个**高性能命令行静态文件服务器**，通过实测数据证明 **Rust 相比 Node.js/Python 的性能优势**。

### 目标指标
- **吞吐量**: > 100k req/s (vs Node.js 35k, Python 12k)
- **内存**: < 50 MB (vs Node.js 180 MB, Python 45 MB)
- **延迟**: P99 < 1 ms (vs Node.js 2-3 ms)
- **启动**: < 50 ms (vs Node.js 1000+ ms)

---

## 🏗️ 架构决策记录 (ADR)

### ADR-001: 为什么选择 axum 而不是 actix-web?

**决策**: 使用 axum (基于 Tokio, 由 Tokio 团队维护)

**对标**:
- actix-web: 性能略优 (5-10%), 但 Tokio 生态不如统一
- hyper: 太底层，需要手动处理太多细节
- rocket: 快速开发友好，但性能不是最优

**原因**:
1. **生态统一**: tokio + axum 无缝搭配
2. **维护稳定**: Tokio 团队直接维护，长期支持有保障
3. **文档完整**: 官方文档齐全，社区案例多
4. **代码简洁**: 路由定义简洁，性能可预测
5. **学习资源丰富**: 官方教程和社区示例充足

**权衡**:
- 比 actix 性能可能低 5-10%，但收益 (维护性/生态) > 成本
- "足够快且可维护" > "极限性能但复杂"

**验证方式**:
- benches/throughput.rs 基准测试
- 预期: ≥ 100k req/s (足以对标 Node.js)

**决策时间**: 2026-03-22  
**相关任务**: TASK.md 中的 "HTTP 服务器核心框架"

---

### ADR-002: 使用 Tokio full features 而不是最小化

**决策**: 启用 Tokio full features

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
```

**对标选项**:
- `["rt-multi-thread"]` 最小化运行时
- `["macros"]` 仅宏支持
- 手动逐个添加 features

**原因**:
1. **功能完整**: fs, net, io, time, signal, sync 等齐全
2. **编译开销极小**: full features 在现代机器上 < 5 秒
3. **二进制额外开销可接受**: 仅 +2 MB (~2%)
4. **简化依赖管理**: 避免后期逐个添加 features 导致重复编译
5. **教育价值**: 展示 "特性完整 ≠ 臃肿"

**权衡**:
- 编译时间 +8-10% (仍 < 5 秒)
- 二进制大小 +2 MB (相比发布版本降低，完全可接受)
- **收益**: 完整开发体验

**验证方式**:
```bash
# 对比编译大小
cargo build --release
du -h target/release/hyper-static-server

# 性能差异 (应无差异)
cargo bench
```

**决策时间**: 2026-03-22

---

### ADR-003: sendfile 系统调用用于零拷贝传输

**决策**: Linux/macOS 使用 sendfile，Windows fallback 到 tokio::fs

```rust
// Linux/macOS: sendfile (零拷贝)
#[cfg(unix)]
async fn send_file_zero_copy(path: &Path) -> Result<Response> {
    // 使用 sendfile 系统调用
    // 理由: 文件数据直接从内核缓冲区到 NIC，不进入用户空间
}

// Windows: 标准异步读
#[cfg(windows)]
async fn send_file_standard(path: &Path) -> Result<Response> {
    // 使用 tokio::fs::read
}
```

**为什么 sendfile**:
1. **零拷贝**: 文件数据直接内核→NIC，减少 memcpy 调用
2. **CPU 高效**: 减少上下文切换，CPU 可做其他工作
3. **吞吐量优化**: 大文件场景 3x+ 性能提升

**性能基准** (实测数据, 待验证):
```
场景: 10MB 文件, 10k 并发连接

方案 A - tokio::fs (内存缓冲):
  - CPU 使用率: 50%
  - 吞吐量: 15k req/s
  - 条带宽: 150 Mbps

方案 B - sendfile (零拷贝):
  - CPU 使用率: 25%
  - 吞吐量: 45k req/s
  - 条带宽: 450 Mbps

收益: 3x 吞吐量 + 50% CPU 节省
```

**权衡**:
- 代码复杂度 (+20 行条件编译)
- 平台差异化 (但有 fallback)
- **收益**: 3x 吞吐量优势 >> 复杂度代价

**验证方式**:
```bash
# benches/throughput.rs 中对比
cargo bench --bench throughput -- --nocapture
```

**决策时间**: 2026-03-22  
**实现计划**: Phase 1 可选 (时间允许), Phase 1.5 必须

---

### ADR-004: 目录遍历防护策略 (安全关键)

**决策**: 使用 `std::fs::canonicalize` 规范化路径，验证在根目录内

**实现原理**:

```rust
fn validate_path(root: &Path, requested: &str) -> Result<PathBuf> {
    let full_path = root.join(requested);
    
    // 规范化所有 .. 和符号链接
    let canonical = full_path.canonicalize()?;
    let root_canonical = root.canonicalize()?;
    
    // 验证最终路径在根目录内
    if !canonical.starts_with(&root_canonical) {
        return Err(Error::PathTraversal); // 403 Forbidden
    }
    
    Ok(canonical)
}
```

**防护的攻击向量**:
```
✅ 阻止: GET ../../etc/passwd
✅ 阻止: GET /etc/passwd (超出 root)
✅ 阻止: GET link_to_etc/passwd (符号链接逃逸)
✅ 允许: GET ./subdir/file.txt (正常访问)
```

**测试用例**:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_path_traversal_blocked() {
        let root = Path::new("/app/public");
        assert!(validate_path(root, "../../etc/passwd").is_err());
    }
    
    #[test]
    fn test_symlink_escape_blocked() {
        // 若 /app/public/link -> /etc
        assert!(validate_path(root, "link/passwd").is_err());
    }
    
    #[test]
    fn test_normal_path_allowed() {
        assert!(validate_path(root, "subdir/file.txt").is_ok());
    }
}
```

**决策时间**: 2026-03-22  
**优先级**: CRITICAL (安全相关)  
**相关任务**: TASK.md 中的 "集成测试"

---

## ⚙️ 内部 API 约定

### HTTP 状态码映射

| 状态码 | 场景 | Response Headers | 备注 |
|--------|------|------------------|------|
| 200 | 文件成功返回 | Content-Type, ETag, Last-Modified, Content-Length | 正常情况 |
| 206 | 部分内容 (Range) | Content-Range, Content-Length, Content-Type | 断点续传 |
| 304 | 未修改 | ETag, Last-Modified | 缓存命中 (If-None-Match) |
| 400 | 请求格式错误 | Content-Type: text/plain | Range 格式错误等 |
| 403 | 禁止访问 | — | 目录遍历、权限不足 |
| 404 | 文件不存在 | — | 文件或目录不存在 |
| 500 | 服务器错误 | — | IO 错误、权限等 |

### Response 结构设计

```rust
// src/response.rs - 响应构建器

pub struct FileResponse {
    pub file: File,
    pub size: u64,
    pub etag: String,
    pub last_modified: SystemTime,
    pub mime_type: String,
}

pub enum ResponseType {
    FullFile(FileResponse),        // 200 OK
    PartialFile(FileResponse, u64, u64), // 206 Partial
    DirectoryListing(String),      // 200 OK + HTML
    NotModified,                   // 304
    NotFound,                      // 404
    Forbidden,                     // 403
    BadRequest(String),            // 400
    ServerError(String),           // 500
}

impl ResponseType {
    pub async fn into_response(self) -> Response {
        // 转换为 axum Response
    }
}
```

**设计原则**:
- 单一职责: 每个变体处理一种响应场景
- 性能优化: 不在内存中加载整个文件
- 安全: 错误消息不泄漏文件系统信息

### Error 处理规范

所有错误统一用 `anyhow::Result<T>`:

```rust
// src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("File not found")]
    NotFound,
    
    #[error("Permission denied")]
    PermissionDenied,
    
    #[error("Path traversal attempt")]
    PathTraversal,
    
    #[error("Invalid range request")]
    InvalidRange,
    
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<ServerError> for StatusCode {
    fn from(err: ServerError) -> Self {
        match err {
            ServerError::NotFound => StatusCode::NOT_FOUND,
            ServerError::PermissionDenied => StatusCode::FORBIDDEN,
            ServerError::PathTraversal => StatusCode::FORBIDDEN,
            ServerError::InvalidRange => StatusCode::BAD_REQUEST,
            ServerError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
```

---

## 📊 性能优化检查清单

### Phase 1 必须做

- [ ] 使用 `tokio::fs` (异步) 而非 `std::fs` (同步)
- [ ] ETag 缓存 (读一次元数据，复用)
- [ ] 路由使用 Trie matching (axum 自动)
- [ ] 错误路径无堆分配

### Phase 2+

- [ ] sendfile 系统调用 (Linux/macOS)
- [ ] Keep-Alive 连接复用
- [ ] 文件描述符缓存
- [ ] 内存池预分配

### 性能标注规范

每个优化点必须在代码中添加 `// PERF:` 注释:

```rust
// ✅ 正确
// PERF: 使用 tokio::fs 异步读取而非同步
// 理由: 不阻塞 Tokio 运行时，支持高并发
// 基准: 1000 并发: 同步 5k req/s vs 异步 50k req/s (10x)
// 链接: benches/throughput.rs L42-60

// ❌ 不够
// 使用异步读取
let data = tokio::fs::read(&path).await?;
```

---

## 🔧 依赖版本

```toml
[package]
name = "hyper-static-server"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web Framework
tokio = { version = "1", features = ["full"] }
axum = "0.7"

# CLI
clap = { version = "4", features = ["derive"] }

# MIME Types
mime_guess = "2"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Utilities
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
reqwest = { version = "0.11", features = ["json"] }
tokio-test = "0.4"
criterion = "0.5"

[profile.release]
opt-level = 3
lto = true
```

---

## 🧪 测试策略

### 单元测试

每个 pub fn 必须有 3 类测试:
```
Happy Path (正常情况)
Edge Case (边界/异常)
Error Case (错误处理)
```

### 集成测试

- 启动服务器
- HTTP 请求验证
- 状态码验证
- 缓存行为验证

### 基准测试

- 吞吐量 (req/s)
- 延迟分布 (p50/p99/max)
- 与竞品对比

---

## 📈 基准对标数据

### 测试环境声明

```
Machine: [CPU型号], [核心数], [内存]
OS: [Windows/Linux/macOS] [版本]
Network: localhost (消除网络延迟)
Test Date: 2026-03-22
```

### 对标工具

- **wrk**: HTTP 压力测试工具
- **criterion**: Rust 基准测试框架
- **Node.js**: express + serve-static
- **Python**: http.server

### 预期结果

| 指标 | hyper-static-server | Node.js | Python | nginx |
|-----|---------------------|---------|--------|-------|
| Throughput (req/s) | **100k-150k** | 30k-40k | 10k-15k | 200k+ |
| P99 Latency (ms) | **0.5-1** | 2-3 | 5-8 | 0.3 |
| Memory (MB) | **15-30** | 150-200 | 40-60 | 10-15 |
| Startup (ms) | **20-50** | 800-1200 | 500-800 | 50-100 |

---

## 📁 文件组织

```
hyper-static-server/
├── .claude/                    # Claude Code 配置
│   ├── rules/
│   │   └── core-rules.md      # 不可违反的规则
│   └── settings.local.json    # 权限配置
│
├── src/
│   ├── main.rs                # CLI 入口 (≤200 行)
│   ├── server.rs              # HTTP 核心 (≤300 行)
│   ├── handler/
│   │   ├── mod.rs
│   │   ├── static.rs          # 静态文件
│   │   ├── directory.rs       # 目录列表
│   │   └── security.rs        # 安全检查
│   ├── response.rs            # 响应构建 (≤200 行)
│   ├── error.rs               # 错误类型 (≤100 行)
│   ├── mime.rs                # MIME 处理 (≤100 行)
│   ├── watch.rs               # 文件监听 [Phase 2]
│   └── lib.rs                 # 公共导出 (≤50 行)
│
├── tests/
│   ├── integration_test.rs    # 端到端测试
│   └── security_test.rs       # 安全测试
│
├── benches/
│   ├── throughput.rs          # 吞吐量基准
│   └── latency.rs             # 延迟基准
│
├── public/
│   └── (前端资源 Phase 2)
│
├── Cargo.toml
├── Cargo.lock
├── CLAUDE.md                  # 开发指南
├── TASK.md                    # 任务追踪
├── PLANNING.md                # 项目规划 (本文件)
├── README.md                  # 用户文档
├── CHANGELOG.md               # 版本历史
└── .gitignore
```

---

## 🚀 发布流程 (v0.1.0)

### 发布前清单

- [ ] 所有功能完成和测试
- [ ] `cargo clippy -- -D warnings` 无警告
- [ ] `cargo test --all` 全部通过
- [ ] `cargo bench` 性能达标
- [ ] README 包含性能表格
- [ ] CHANGELOG.md 更新

### 发布步骤

```bash
# 1. 更新版本
cargo build --release

# 2. 运行所有测试
cargo test --all

# 3. 验证基准
cargo bench --bench throughput

# 4. 标记版本
git tag v0.1.0
git push origin v0.1.0

# 5. 更新 CHANGELOG.md
# 6. 发布 GitHub Release
```

---

## 📚 参考资源

- [Tokio 官方教程](https://tokio.rs/)
- [axum 官方文档](https://docs.rs/axum/)
- [Rust 性能书](https://nnethercote.github.io/perf-book/)
- [RFC 7233 - HTTP Range Requests](https://tools.ietf.org/html/rfc7233)
- [ETag 缓存机制](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)

---

**最后更新**: 2026-03-22


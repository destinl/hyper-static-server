# hyper-static-server 开发指南

**项目**: 高性能 Rust 静态文件服务器  
**语言**: Rust (后端) + TypeScript (前端)  
**开始日期**: 2026-03-22

---

## 🔄 项目认知

### 必读文档
- **[PLANNING.md](PLANNING.md)** - 架构决策 (ADR) 和为什么用 axum、tokio 等
- **[TASK.md](TASK.md)** - 当前任务状态，完成后立即标记 ✅
- **[README.md](README.md)** - 项目概述和快速开始
- **[.claude/rules/core-rules.md](.claude/rules/core-rules.md)** - 不可违反的规则

### 核心目标
1. 实现高性能静态文件服务 (> 100k req/s)
2. 证明 Rust 相比 Node.js 的性能优势 (3x+ 吞吐量, 4x+ 内存优势)
3. 代码质量可维护和可验证

---

## 🧱 代码模块化规则

### 单文件最大行数限制

**目标**: 每个 Rust 源文件 ≤ 300 行 (中等复杂度)

**超过限制时的处理**:
- 若 `server.rs` 达到 350 行 → 拆分为 `server/` 目录:
  ```
  server/
  ├── mod.rs         (导出接口)
  ├── routing.rs     (路由定义)
  └── handler.rs     (请求处理)
  ```

- 若 `handler.rs` 达到 280 行 → 拆分为:
  ```
  handler/
  ├── mod.rs         (导出)
  ├── static.rs      (静态文件逻辑)
  ├── directory.rs   (目录列表)
  └── security.rs    (安全检查)
  ```

### 模块组织原则

```
src/
├── lib.rs              # 公共导出 (≤ 50 行)
├── main.rs             # CLI 入口 (≤ 200 行)
├── server.rs           # HTTP 服务核心 (≤ 300 行)
├── handler/
│   ├── mod.rs
│   ├── static.rs       # 静态文件处理
│   ├── directory.rs    # 目录列表
│   └── security.rs     # 安全检查
├── response.rs         # 响应构建 (≤ 200 行)
├── error.rs            # 统一错误类型 (≤ 100 行)
├── mime.rs             # MIME 处理 (≤ 100 行)
└── watch.rs            # 文件监听 [Phase 2] (≤ 150 行)
```

### 模块职责

| 模块 | 职责 | 不能做 |
|-----|------|--------|
| main.rs | CLI 解析 + 服务启动 | 请求处理逻辑 |
| server.rs | 路由定义 + 监听配置 | 业务逻辑细节 |
| handler | 请求处理分发 | HTTP 响应组装 |
| response.rs | HTTP 响应构建 | 文件读取 |
| error.rs | 错误类型定义 | 错误处理逻辑 |

---

## 🧪 测试规范

### 单元测试 (必须)

每个 `pub fn` 必须有 **3 个测试用例**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_etag_happy_path() {
        // ✅ 正常情况: 普通文件
        let etag = generate_etag(12345, 1024);
        assert!(!etag.is_empty());
        assert!(etag.contains('-'));
    }

    #[test]
    fn test_generate_etag_edge_case_zero_size() {
        // ⚠️ 边界情况: 零字节文件
        let etag = generate_etag(0, 0);
        assert_eq!(etag, "0-0");
    }

    #[test]
    #[should_panic]
    fn test_generate_etag_error_case() {
        // ❌ 错误情况: 无效输入
        // (或返回 Err 而不是 panic)
    }
}
```

**规则**:
- Happy Path: 验证正常行为
- Edge Case: 空文件、超大文件、特殊字符等
- Error Case: 权限不足、不存在等

### 集成测试 (必须)

放在 `tests/integration_test.rs`:

```bash
cargo test --test integration_test
```

内容:
- 启动完整服务器
- 用 `reqwest` 发送 HTTP 请求
- 验证状态码、头、内容

### 基准测试 (必须)

放在 `benches/`:

```bash
cargo bench --bench throughput
```

必须包含:
- 吞吐量 (req/s)
- P99 延迟 (ms)
- 与 Node.js/Python 的对比

---

## 📚 文档规范

### 代码注释

所有 `pub` 函数都必须有 doc comment:

```rust
/// 生成 ETag 用于 HTTP 缓存验证
/// 
/// 使用文件的修改时间和大小计算 ETag，格式为 "mtime-size" 的十六进制表示。
///
/// # Arguments
/// * `mtime` - 文件修改时间戳
/// * `size` - 文件大小（字节）
///
/// # Returns
/// 十六进制格式的 ETag 字符串
///
/// # Examples
/// ```
/// let etag = generate_etag(1234567890, 4096);
/// assert_eq!(etag, "499602d2-1000");
/// ```
pub fn generate_etag(mtime: u64, size: u64) -> String {
    format!("{:x}-{:x}", mtime, size)
}
```

### 性能优化注释

所有优化必须标注**为什么**:

```rust
// ✅ 正确示例
// PERF: 使用 sendfile 零拷贝传输 (Linux/macOS)
// 理由: 文件数据直接从内核到 NIC，避免内存缓冲
// 基准: 10MB 文件, 10k 并发:
//   - tokio::fs read: 50% CPU, 15k req/s
//   - sendfile: 25% CPU, 45k req/s (3x 提升)
// 链接: benches/throughput.rs L45
#[cfg(unix)]
async fn send_file_zero_copy(file: File) -> Result<Response> {
    // 实现...
}

// ❌ 不够的示例
// 使用异步读取
let data = tokio::fs::read(&path).await?;
```

### README.md 结构

```markdown
# hyper-static-server

## 快速开始 (< 50 行)
编译、运行、验证

## 性能基准 (表格)
与 Node.js/Python 的对比数据

## 功能文档 (详细)
参数说明、API、例子

## 开发指南
测试、基准、贡献流程
```

---

## 🔐 安全检查清单

### Phase 1 (本期必须实现)

- [ ] **目录遍历防护**
  ```rust
  // 使用 canonicalize 规范化路径
  let canonical = path.canonicalize()?;
  assert!(canonical.starts_with(root));
  ```
  - 单元测试验证 `../../etc/passwd` 被拒绝
  - 测试用例见 `tests/security_test.rs`

- [ ] **符号链接处理**
  - 命令行选项: `--follow-symlinks` (默认 false)
  - README 中文档说明风险

- [ ] **错误信息暴露检查**
  - 若访问 `/var/log/app.log`，返回 `404` 而非文件路径
  - 测试: 访问不存在文件，验证无路径信息

### Phase 2+ (后续)

- CORS 配置验证
- Basic Auth 认证
- 限流 (DDoS 防护)

---

## ⚠️ AI 助手约束

### DO ✅
- 遵循 CLAUDE.md 所有规范
- 遵循 PLANNING.md 架构决策
- 完成任务后立即在 TASK.md 标记 ✅
- 遇到不确定，添加到 TASK.md Discovered
- 性能优化必须有 // PERF: 注释 + 基准链接

### DON'T ❌
- 不要假设 crates.io 中没有验证的依赖
- 不要假设 sendfile 在 Windows 可用
- 不要生成 > 300 行的单个文件
- 不要删除代码除非 TASK.md 明确要求
- 不要跳过 cargo clippy / cargo fmt / cargo test

---

## ✅ 任务完成流程

### 启动任务
1. 在 TASK.md 找到任务
2. 标记为进行中: `[IN PROGRESS]` 或加时间戳

### 编写代码
1. 遵循模块化原则 (≤ 300 行/文件)
2. 添加 doc comment (所有 pub fn)
3. 添加单元测试 (happy/edge/error 三种)
4. 性能关键路径添加 // PERF: 注释

### 验证
```bash
# 格式检查
cargo fmt --check

# Lint 检查
cargo clippy -- -D warnings

# 测试
cargo test --all

# 性能基准
cargo bench --bench throughput
```

### 标记完成
1. 在 TASK.md 标记: `[x] 任务名 (完成: 2026-03-22 18:30)`
2. 更新 CHANGELOG.md (若有重大功能)
3. 提交 commit

### 发现新问题
1. 添加到 TASK.md Discovered 部分
2. 备注上下文信息
3. 标注优先级 (HIGH/MEDIUM/LOW)

---

## 🏗️ 首周开发日程建议

| 天 | 重点 | 检查清单 |
|----|------|--------|
| D1 | 初始化 | [ ] Cargo.toml 完成，依赖验证 |
| D2 | 核心架构 | [ ] server.rs 框架完成 |
| D3-4 | 功能实现 | [ ] 静态文件、目录列表、错误处理 |
| D5 | 单元测试 | [ ] 所有 pub fn 都有 3 类测试 |
| D6 | 集成测试 | [ ] 所有 HTTP 状态码验证 |
| D7 | 性能优化 + 文档 | [ ] 基准测试对比，README 完整 |

---

## 📞 问题排查

### 错误: "文件超过 300 行"
→ 查看 [模块组织原则](#模块组织原则)，拆分为子模块

### 错误: "性能下降 > 5%"
→ 用 `cargo bench` 对比旧版本，在 commit message 解释原因

### 错误: "pub fn 没有文档"
→ 添加 `/// 说明` doc comment，见 [代码注释](#代码注释) 示例

### 问题: "不确定是否用 sendfile"
→ 添加到 TASK.md Discovered，我会确认后推进

---

## 🎓 学习资源

- [Tokio 官方文档](https://tokio.rs/)
- [axum 官方文档](https://docs.rs/axum/)
- [Rust 性能书](https://nnethercote.github.io/perf-book/)
- [ETag 和缓存机制](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)

---

**记住**: 每个规则都是为了保证代码质量和性能。遇到疑问，优先查阅文档而非猜测。✨


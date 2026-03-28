# 项目配置说明

## 关键文件位置

### Claude Code 规则和配置
- **`.claude/settings.local.json`** - 权限配置（cargo, rustup, git, curl等）
- **`.claude/rules/core-rules.md`** - 不可违反的规则（关键！）
  - 文件大小 ≤ 300 行
  - 性能基准必须保证
  - 测试覆盖要求
  - 安全检查清单

### 开发指南
- **`CLAUDE.md`** - AI 助手的主要工作手册
  - 模块化规则
  - 测试规范
  - 文档规范
  - 安全检查
  - AI 约束

- **`PLANNING.md`** - 项目架构和设计决策
  - ADR (Architecture Decision Records)
  - 为什么选择 axum/tokio
  - 性能优化策略
  - 基准测试对标

- **`TASK.md`** - 任务追踪和进度
  - 当前待做任务
  - 已完成任务
  - Discovered During Work (发现的新问题)
  - 时间估算

- **`README.md`** - 用户文档
  - 快速开始
  - 性能基准表格
  - 参数说明
  - 使用示例

- **`CHANGELOG.md`** - 版本历史
  - 每个版本的功能
  - 性能指标
  - 破坏性变更

### 源代码
- **`src/main.rs`** - CLI 入口（≤ 200 行）
- **`src/lib.rs`** - 公共导出（≤ 50 行）
- **`src/server.rs`** - HTTP 服务核心（≤ 300 行）
- **`src/handler/`** - 请求处理（拆分为 static, directory 等）
- **`src/response.rs`** - 响应构建（≤ 200 行）
- **`src/error.rs`** - 统一错误类型（≤ 100 行）
- **`src/mime.rs`** - MIME 处理（≤ 100 行）

### 测试
- **`tests/integration_test.rs`** - 端到端测试
- **`benches/throughput.rs`** - 吞吐量基准
- **`benches/latency.rs`** - 延迟基准

### 其他
- **`Cargo.toml`** - Rust 项目配置
- **`.gitignore`** - Git 忽略配置
- **`public/`** - 前端资源（Phase 2）

---

## 如何与 Claude Code 配合

### 首次使用

1. **在 VS Code 中打开项目**
   ```bash
   code hyper-static-server
   ```

2. **启动 Claude Code 新会话**
   - Ctrl+Shift+I (Windows/Linux)
   - Cmd+Shift+I (macOS)

3. **复制以下提示词到 Claude Code**
   ```
   你好，我想用 Context Engineering 方法实现 hyper-static-server 项目。
   
   我已经创建了以下配置：
   - .claude/settings.local.json (权限配置)
   - .claude/rules/core-rules.md (不可违反的规则)
   - CLAUDE.md (开发指南)
   - TASK.md (任务追踪)
   - PLANNING.md (项目规划)
   
   请按照这些文件实现项目。规则：
   1. 每个 Rust 文件 ≤ 300 行（如果需要可调整为 400-500）
   2. 所有 pub fn 都要有文档注释
   3. 每个 pub fn 都要有 3 种测试 (happy/edge/error)
   4. 性能优化必须标注 // PERF: 注释
   5. 完成每个任务后，我会在 TASK.md 标记 ✅
   6. 遇到不确定的地方，添加到 TASK.md Discovered 部分
   
   现在，请检查 TASK.md，告诉我第一个该做的任务是什么？
   ```

4. **Claude 会**
   - 读取所有配置
   - 遵循 CLAUDE.md 和 core-rules.md 的规范
   - 生成高质量的代码
   - 提醒你更新 TASK.md

### 每个任务完成后

1. **在 TASK.md 标记完成**
   ```
   从: - [ ] 任务名
   改为: - [x] 任务名 (完成: 2026-03-22 18:30)
   ```

2. **告诉 Claude**
   ```
   @Claude 我已经在 TASK.md 标记了任务完成。
   请检查 TASK.md，看下一个该做的任务是什么？
   ```

3. **Claude 会**
   - 读取更新的 TASK.md
   - 继续下一个任务
   - 确保所有完成的任务达到标准

### 遇到问题时

1. **如果不确定某个设计决策**
   ```
   在 TASK.md Discovered 部分添加：
   - [ ] 问题描述 (影响: 哪个任务)
   ```

2. **Claude 会**
   - 看到新的 Discovered 项
   - 提出建议或等待你的确认

3. **你确认后**
   ```
   从: - [ ] 问题描述
   改为: - [x] 问题描述 (决策: xxx, 已实现)
   ```

---

## 快速参考

### CLI 运行测试
```bash
# 格式检查
cargo fmt --check

# Lint 检查
cargo clippy -- -D warnings

# 单元测试
cargo test --lib

# 集成测试
cargo test --test integration_test

# 性能基准
cargo bench --bench throughput

# 全部检查
cargo fmt --check && cargo clippy -- -D warnings && cargo test --all && cargo bench
```

### 文件大小检查
```bash
# 查看所有源文件行数
find src -name "*.rs" | xargs wc -l
```

### 性能关键路径标注示例
```rust
// ✅ 正确
// PERF: 使用 tokio::fs 异步而非同步
// 理由: 不阻塞 Tokio 运行时
// 基准: benches/throughput.rs L45
// 发现: 10k 并发下 5x 吞吐量提升

// ❌ 不够
// 使用异步读取
```

---

## 项目地图

```
设置阶段:
  1. 打开项目并阅读本文件
  2. 启动 Claude Code 新会话
  3. 复制上面的提示词

开发阶段:
  4. Claude 生成代码
  5. 你验证并在 TASK.md 标记
  6. 重复 4-5 直到全部完成

发布阶段:
  7. 运行所有测试和基准
  8. 更新 README 和 CHANGELOG
  9. git tag v0.1.0
  10. 发布到 GitHub
```

---

## 关键数字

- **目标吞吐量**: ≥ 100k req/s
- **目标内存**: ≤ 50 MB
- **文件大小限制**: ≤ 300 行
- **测试覆盖**: 3 种/函数
- **性能回退阈值**: > 5%

---

**记住**: 这个项目的目标是证明 Rust 的性能优势。每个决策都是为了实现这个目标。✨


# hyper-static-server 核心规则

## 🚨 绝不违反的规则

### 1. 代码质量不可妥协
- **单文件最大 300 行**: 若超过，必须拆分模块
- **所有 pub 函数必须有文档注释** (/// doc comment)
- **无 clippy 警告**: `cargo clippy -- -D warnings` 必须通过
- **无未使用代码**: 删除所有 dead code

### 2. 性能指标是保证
- **基准测试必须运行**: 每个 Commit 前运行 `cargo bench`
- **性能下降 > 5% 必须解释**: 在 commit message 中说明原因
- **内存占用目标**: < 50MB (vs Node.js 180MB 的对比)
- **吞吐量目标**: > 100k req/s (vs Node.js 35k)

### 3. 测试覆盖率不可低
- **单元测试**: 每个 pub fn 必须有正常/边界/错误三种测试
- **集成测试**: tests/ 目录中必须验证端到端流程
- **基准测试**: benches/ 目录必须提供与竞品对比

### 4. 安全第一
- **目录遍历防护**: 必须验证 canonicalize 不超出根目录
- **符号链接处理**: 添加 --follow-symlinks 配置
- **错误信息**: 绝不泄漏文件系统信息到客户端

### 5. 文档即代码
- **性能优化必须有注释**: 
  ```rust
  // PERF: 使用 sendfile 零拷贝传输
  // 基准: 3x 吞吐量提升 (见 benches/throughput.rs)
  ```
- **架构决策记录在 PLANNING.md**
- **任务完成标记在 TASK.md**

---

## 🚦 代码审查检查清单

提交前必须通过：

- [ ] `cargo fmt --check` (格式检查)
- [ ] `cargo clippy -- -D warnings` (Lint)
- [ ] `cargo test --all` (所有测试)
- [ ] `cargo bench --bench throughput` (性能基准)
- [ ] 文件大小都 ≤ 300 行
- [ ] 性能基准数据链接正确
- [ ] TASK.md 已更新
- [ ] git commit message 清晰

---

## ⚠️ AI 助手必须遵守的约束

### DO ✅
- 遵循 CLAUDE.md 的所有规范
- 遵循 PLANNING.md 的架构决策
- 完成任务后立即更新 TASK.md
- 不确定时，添加到 TASK.md Discovered 部分
- 性能优化必须有 // PERF: 注释

### DON'T ❌
- 不要猜测 crates.io 上没有的依赖
- 不要假设 sendfile 在 Windows 可用（需要 fallback）
- 不要跳过测试或基准测试
- 不要生成 > 300 行的单个文件
- 不要删除代码除非 TASK.md 明确要求

---

## 📋 模块大小规范

| 模块 | 目标行数 | 超过时 |
|-----|---------|--------|
| main.rs | ≤ 200 | 拆分为独立模块 |
| server.rs | ≤ 300 | 路由逻辑 → routing.rs |
| handler.rs | ≤ 250 | 静态文件 → static.rs, 目录 → directory.rs |
| response.rs | ≤ 200 | ETag → etag.rs, Range → range.rs |
| 其他模块 | ≤ 150 | 按职责细分 |

---

## 🔍 性能监测规矩

### 关键指标（MUST TRACK）
- 吞吐量 (req/s)
- P99 延迟 (ms)
- 内存占用 (MB)
- 启动时间 (ms)

### 基准对标
```
与以下竞品对标（见 benches/ 目录）:
- Node.js + express-static
- Python http.server
- nginx (基线)
```

### 性能回退处理
```
若性能下降 > 5%:
1. cargo bench 对比旧版本
2. 在 PLANNING.md 记录原因
3. 若不可接受，必须优化回复
4. commit message: "perf(handler): optimize X, +5% throughput"
```


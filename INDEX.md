# 📑 hyper-static-server 文档导航

> 快速查找所有文档和文件

---

## 🚀 快速开始（必读）

| 优先级 | 文件 | 哈 | 阅读时间 |
|--------|------|-----|---------|
| ⭐⭐⭐⭐⭐ | **[STARTED_HERE.md](STARTED_HERE.md)** | 项目完成概览 | 5 分钟 |
| ⭐⭐⭐⭐⭐ | **[.claude/PROJECT_SETUP_GUIDE.md](.claude/PROJECT_SETUP_GUIDE.md)** | 如何使用 Claude Code | 5 分钟 |
| ⭐⭐⭐⭐⭐ | **[CLAUDE.md](CLAUDE.md)** | 开发规范和约束 | 10 分钟 |
| ⭐⭐⭐⭐⭐ | **[.claude/rules/core-rules.md](.claude/rules/core-rules.md)** | 不可违反的规则 | 5 分钟 |

---

## 📋 项目管理

| 文件 | 用途 | 更新频率 |
|-----|------|---------|
| **[TASK.md](TASK.md)** | 任务追踪、进度管理 | **每完成一个任务** |
| **[PLANNING.md](PLANNING.md)** | 架构决策 (ADR) | 每周 |
| **[CHANGELOG.md](CHANGELOG.md)** | 版本历史和功能 | 每个版本 |

---

## 📚 参考文档

| 文件 | 用途 | 对象 |
|-----|------|------|
| **[README.md](README.md)** | 用户文档、使用指南 | 项目使用者 |
| **[CLAUDE.md](CLAUDE.md)** | 开发规范、代码标准 | 开发者 + Claude AI |
| **[PLANNING.md](PLANNING.md)** | 架构设计、为什么这样做 | 开发者 + 新成员 |

---

## 🛠️ 配置和项目文件

| 文件 | 用途 |
|-----|------|
| `Cargo.toml` | Rust 依赖和项目配置 |
| `.gitignore` | Git 忽略规则 |
| `.claude/settings.local.json` | Claude Code 权限配置 |
| `.claude/rules/core-rules.md` | Claude Code 行为规则 |
| `.claude/PROJECT_SETUP_GUIDE.md` | 项目使用指南 |

---

## 📁 源代码目录

| 路径 | 用途 | 阶段 |
|-----|------|------|
| `src/main.rs` | CLI 入口 (≤ 200 行) | Phase 1 |
| `src/lib.rs` | 公共导出 (≤ 50 行) | Phase 1 |
| `src/server.rs` | HTTP 服务核心 (≤ 300 行) | Phase 1 |
| `src/handler/` | 请求处理模块 | Phase 1 |
| `src/response.rs` | 响应构建 (≤ 200 行) | Phase 1 |
| `src/error.rs` | 错误定义 (≤ 100 行) | Phase 1 |
| `src/mime.rs` | MIME 处理 (≤ 100 行) | Phase 1 |
| `src/watch.rs` | 文件监听 (≤ 150 行) | Phase 2 |

---

## 🧪 测试和基准

| 路径 | 用途 |
|-----|------|
| `tests/integration_test.rs` | 集成测试框架 |
| `benches/throughput.rs` | 吞吐量基准测试 |
| `benches/latency.rs` | 延迟基准测试 |

---

## 🚀 启动脚本

| 文件 | 系统 |
|-----|------|
| `quick-start.bat` | Windows |
| `quick-start.sh` | Linux/macOS |

---

## 📖 按场景查找文档

### "我想立即开始开发"
→ [STARTED_HERE.md](STARTED_HERE.md)

### "我想了解开发规范"
→ [CLAUDE.md](CLAUDE.md)

### "我想知道下一个任务是什么"
→ [TASK.md](TASK.md)

### "我想了解为什么选择某个技术"
→ [PLANNING.md](PLANNING.md) (ADR 部分)

### "我想了解如何使用这个项目"
→ [README.md](README.md)

### "我想帮助别人理解这个项目"
→ [.claude/PROJECT_SETUP_GUIDE.md](.claude/PROJECT_SETUP_GUIDE.md)

### "我想了解 Claude Code 如何工作"
→ [.claude/rules/core-rules.md](.claude/rules/core-rules.md)

---

## 📊 文件统计

| 类型 | 数量 | 总行数 |
|-----|------|--------|
| 配置文件 | 5 | 100 |
| 文档 | 8 | 3000+ |
| 源代码占位符 | 2 | 20 |
| 测试框架 | 3 | 50 |
| **总计** | **18** | **3000+** |

---

## 🎯 阅读建议

### 第一次接触项目（10 分钟）
1. [STARTED_HERE.md](STARTED_HERE.md)
2. [.claude/PROJECT_SETUP_GUIDE.md](.claude/PROJECT_SETUP_GUIDE.md)
3. [quick-start.bat](quick-start.bat) 或 [quick-start.sh](quick-start.sh)

### 开始开发前（15 分钟）
1. [CLAUDE.md](CLAUDE.md) - 全部阅读
2. [TASK.md](TASK.md) - 了解当前任务
3. [.claude/rules/core-rules.md](.claude/rules/core-rules.md) - 重点阅读

### 深入理解项目（30 分钟）
1. [PLANNING.md](PLANNING.md) - ADR 部分
2. [README.md](README.md) - 完整阅读
3. [CHANGELOG.md](CHANGELOG.md) - 版本历史

### 遇到问题时
1. [TASK.md](TASK.md) - Discovered 部分
2. [PLANNING.md](PLANNING.md) - 相关 ADR
3. [CLAUDE.md](CLAUDE.md) - 搜索关键词

---

## 🔗 文档关系图

```
STARTED_HERE.md (开始)
    ↓
.claude/PROJECT_SETUP_GUIDE.md (如何用)
    ↓
CLAUDE.md (规范) + TASK.md (任务)
    ↓
PLANNING.md (架构) + README.md (用户)
    ↓
源代码开发
```

---

## ✅ 检查清单

启动 Claude Code 前，确保你：

- [ ] 阅读了 STARTED_HERE.md
- [ ] 理解了 CLAUDE.md 的规范
- [ ] 知道如何在 TASK.md 标记任务
- [ ] 了解了不可违反的 rules/core-rules.md
- [ ] 准备好启动 Claude Code 新会话

---

**现在就打开 [STARTED_HERE.md](STARTED_HERE.md) 开始吧！** 🚀


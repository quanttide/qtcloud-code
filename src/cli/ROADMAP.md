# ROADMAP — qtcloud-code-cli

## 定位

多语言代码静态分析 CLI，聚焦**可检测、可复现、可自动化**的代码问题。
不依赖 LLM，纯规则引擎 + AST 分析 + 编译器集成。

## 阶段

### P0 — 已完成 ✅

- [x] CLI 命令框架（`review` / `list-rules`）
- [x] 多语言解析：Rust / Python / Go / Dart / TypeScript (TSX)
- [x] 5 检测器：过长函数、unsafe 块、过长参数列表、未使用变量、缺失测试
- [x] 配置系统：`.quanttide/code/contract.yaml` + `--rules` 过滤
- [x] 输出格式：终端 / JSON / STATUS.md
- [x] 自举验证 + 77 测试 + 95% 覆盖率
- [x] 发布流水线：`cli/v*` tag → build-cli → release-cli

### P1 — `contract` 命令升级

`list-rules` 升级为 `contract` 子命令，统一管理契约配置。

```
qtcloud-code contract init        # 创建默认 .quanttide/code/contract.yaml
qtcloud-code contract list        # 列出可用规则（当前 list-rules 功能）
qtcloud-code contract validate    # 校验配置 vs 实际规则
qtcloud-code contract diff        # 对比配置与项目实际状态
```

区别：

| 当前 | 目标 |
|------|------|
| `list-rules` 仅打印规则列表 | `contract list` 可输出 JSON，支持 `--enabled` 过滤 |
| 无初始化向导 | `contract init` 交互式创建配置 |
| 无校验 | `contract validate` 发现废弃规则、缺失排除等 |
| 规则硬编码在 main.rs | 规则元数据可抽取为独立模块 |

### P2 — 检测器深度

- [ ] Go / Dart / TypeScript 专用检测器（当前只有 Rust 的 unsafe_block）
- [ ] 嵌套过长函数检测（回调地狱、箭头函数链）
- [ ] 文件级忽略机制（行级 `/ /qtcloud-code ignore` + 配置级 exclude）
- [ ] 跨文件检测：重复代码、循环依赖

### P3 — 项目级增强

- [ ] 增量扫描（基于 git diff，只分析变更文件）
- [ ] 基线模式（`--baseline baseline.json`，仅报告新增问题）
- [ ] 规则推荐：基于项目语言和规模自动推荐启用规则
- [ ] 多语言通用 `unsafe` / 等效关键词检测

## 非目标

- 不做自动修复（只检测，不修改）
- 不做全语义级分析（类型推断、数据流分析）
- 不依赖 LLM 或外部 API

# ROADMAP — qtcloud-code-cli

## 定位

3R 代码审查 CLI：**review（已发布）→ reflect（开发中）→ refactor（预览版）**。

纯规则引擎 + tree-sitter AST 分析 + cargo check。LLM 集成在 ROADMAP 上但尚未实现。

## P0 — 已完成 ✅

- [x] CLI 框架（`review` / `list-rules`）
- [x] 多语言解析：Rust / Python / Go / Dart / TypeScript (TSX)
- [x] 5 检测器：过长函数、unsafe 块、过长参数列表、未使用变量、缺失测试
- [x] 配置系统：`.quanttide/code/contract.yaml` + `--rules`
- [x] 输出格式：终端 / JSON / STATUS.md
- [x] 自举验证 + 119 测试
- [x] 发布流水线：`cli/v*` tag → crates.io

## P1 — `--llm` 模式（LLM 审查）

引入 `--llm` 标志，LLM 对规则引擎的 finding 做二次审查。

```sh
qtcloud-code review .              # 规则引擎（当前行为）
qtcloud-code review . --llm        # 规则引擎 + LLM 审查
```

### LLM 审查内容

- 对每个 finding 做优先级排序、上下文追加、误报标记
- 纯 LLM 规则：安全漏洞、并发 bug、逻辑错误（规则引擎无法检测的）
- 跨 finding 根因分析（reflect）
- 项目级健康摘要

### 需要实现

- [ ] `--llm` 标志和 LLM 客户端接口
- [ ] finding 增强：LLM 输出叠加到原始 finding
- [ ] 纯 LLM 规则定义框架
- [ ] 成本控制：只对有 finding 的文件发起调用
- [ ] 缓存：同一文件同一版本的结果缓存

## P2 — `--mode deep`（LLM 修复）

引入 `--mode deep`，LLM 在审查基础上生成修复 patch。

```sh
qtcloud-code review . --mode deep           # dry-run（显示 diff）
qtcloud-code review . --mode deep --apply   # 确认写入
```

### 安全机制（从实验室 refactor/safety.rs 集成）

- [ ] `Patch` 结构：文件、行范围、新旧文本
- [ ] `dry-run`：生成 diff，不写文件
- [ ] `--apply`：确认写入，验证 old_text 匹配
- [ ] 自动验证：编译 + 测试通过才确认
- [ ] 验证失败回退
- [ ] `rollback` 回滚
- [ ] `OperationLog` 操作日志

### 符号表（从实验室 refactor/rename.rs 集成）

- [ ] `SymbolTable`：函数定义→调用点映射
- [ ] `build_symbol_table()`：扫描项目构建符号表
- [ ] `rename_symbol()`：生成替换映射

## P3 — `contract` 命令

`list-rules` 升级为 `contract` 子命令。

```sh
qtcloud-code contract init        # 创建默认配置
qtcloud-code contract list        # 列出可用规则（JSON 支持）
qtcloud-code contract validate    # 校验配置 vs 实际规则
qtcloud-code contract diff        # 对比配置与项目实际状态
```

## 非目标

- 不做全语义级分析（类型推断、数据流分析）
- LLM 集成前不做纯 LLM 代码审查

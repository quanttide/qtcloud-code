# ROADMAP — qtcloud-code-cli

## 定位

多语言代码静态分析 CLI，聚焦**可检测、可复现、可自动化**的代码问题。不依赖 LLM，纯规则引擎 + AST 分析。

## 阶段

### P0 — CLI 骨架 & 单语言 MVP

- [ ] CLI 命令框架（`scan`/`check`/`list-rules`）
- [ ] tree-sitter 集成，支持 Rust 解析
- [ ] 第一个检测器：无用依赖 / 未使用变量（Rust）
- [ ] 输出格式：JSON + 终端表格

### P1 — 多语言扩展

- [ ] 语言解析器抽象层（`LanguageParser` trait）
- [ ] Python 支持（tree-sitter-python）
- [ ] Go 支持（tree-sitter-go）
- [ ] Dart 支持（tree-sitter-dart）
- [ ] TypeScript/JavaScript 支持（tree-sitter-typescript）
- [ ] 通用检测器：过长函数、过长参数列表、重复代码

### P2 — 规则系统

- [ ] 规则注册与配置（`--rules` / `.qtcloud-code.toml`）
- [ ] 规则分类：正确性 / 性能 / 可维护性 / 风格
- [ ] 忽略机制（行级注释 / 文件级配置）
- [ ] 自定义规则 DSL 或插件接口

### P3 — 生产就绪

- [ ] CI 集成（GitHub Action、pre-commit hook）
- [ ] 增量扫描（只分析 diff）
- [ ] 基线模式（仅报告新增问题）
- [ ] 多线程并行扫描

## 非目标

- 不做自动修复（只检测，不修改）
- 不做语义级分析（类型推断、数据流分析）
- 不依赖 LLM 或外部 API

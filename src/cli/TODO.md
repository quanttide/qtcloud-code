# TODO — qtcloud-code-cli

> 当前版本: `cli/v0.2.0` · P0 已发布 · P1 代码完成 · P2 未开始

## 🔴 紧急（阻塞当前迭代）

- [ ] **提交推送 reflect 变更** — Cargo.toml / main.rs / tests/reflect.rs / STATUS.md / TODO.md / ROADMAP.md

## 🟡 P1.5 — reflect 完善（当前迭代）

- [ ] **消除编译 warning** — `_tree`（graph handler 未使用解析树）、`_json`（未实现 JSON 输出）
- [ ] **移除 `_json` 死参数** — 各 handler 签名中 `_json: bool` 未使用，clap 也未定义 `--json` flag。要么实现，要么移除
- [ ] **修复 `unsafe` 检测** — `run_reflect_suggest` 中 `t.contains("unsafe")` 匹配范围过大，可能误报 `unsafe fn`、`unsafe trait` 等声明。至少排除 `unsafe fn` / `unsafe trait` / `unsafe impl` 前缀
- [ ] **slice 改用 AST 追溯** — 当前是行级倒序收集，非真正的 tree-sitter 回溯。至少做到按函数范围限定追溯范围，而非从文件开头截取

## 🔵 P2 — `--llm` 集成（规划中）

- [ ] LLM 客户端 + `--llm` 标志
- [ ] 统一 prompt（dismiss + discover）作为默认 prompt
- [ ] 规则引擎验证层：LLM 诊断 → 匹配 VerificationRule
- [ ] 双通道策略：全函数分析 + 片段聚焦
- [ ] 置信度标注（证据锚定率 + 交叉验证）

## ⚪ 技术债务

- [ ] **reflect handler 提取** — 4 个 handler 目前 inline 在 `main.rs`，应提取到独立模块 `src/reflect/`
- [ ] **`make_parser` 与 `create_parsers` 合并** — 两套不同的 parser 创建方式，应统一
- [ ] **`trace` 自动查找声明跨语言** — 当前仅 Rust `let var` 模式匹配，Python/Go/TS 不生效
- [ ] **`graph` 生成真实调用图** — 当前只匹配 `fn` 行，调用/被调用数硬编码 0
- [ ] **test_graph_typescript / test_slice_python 断言过松** — 同时接受 exit 0 和 1，应确定正确值后收紧
- [ ] **CHANGELOG.md 记录** — 添加 `reflect` 子命令和 12 测试的变更记录

## 📋 提交清单（当前）

- [ ] `git add src/cli/Cargo.toml src/cli/src/main.rs src/cli/tests/reflect.rs src/cli/ROADMAP.md src/cli/STATUS.md src/cli/TODO.md`
- [ ] `git commit -m "feat: add reflect subcommand with slice/trace/graph/suggest + 12 tests"`
- [ ] `git push`

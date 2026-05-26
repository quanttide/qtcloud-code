# TODO — qtcloud-code-cli

> 当前版本: `cli/v0.2.0` · P0 已发布 · P1 已提交+推送

## 🟢 已完成（当前迭代）

- `_json` 死参数移除 ✅ · 编译 warning 消除 ✅ · unsafe 检测条件修复 ✅
- slice AST 函数作用域追溯 ✅ · make_parser 复用（refactor rename） ✅
- trace 跨语言声明查找（Python/Go） ✅ · CHANGELOG v0.2.1 记录 ✅

## 🔵 P2 — `--llm` 集成（规划中）

- [ ] LLM 客户端 + `--llm` 标志
- [ ] 统一 prompt（dismiss + discover）作为默认 prompt
- [ ] 规则引擎验证层：LLM 诊断 → 匹配 VerificationRule
- [ ] 双通道策略：全函数分析 + 片段聚焦
- [ ] 置信度标注（证据锚定率 + 交叉验证）

## ⚪ 技术债务

- [ ] **`make_parser` 与 `create_parsers` 合并** — 两套不同的 parser 创建方式，`create_parsers` 使用 `LanguageParser` trait + `ParseResult`，`make_parser` 返回裸 `tree_sitter::Parser`。虽然类型不同，但可提取公共 `resolve_language` 函数共享 ext→language 映射
- [ ] **`graph` 生成真实调用图** — 当前只匹配 `fn` 行，调用/被调用数硬编码 0。可遍历 AST 搜索函数调用
- [ ] **test_graph_typescript / test_slice_python 断言过松** — 同时接受 exit 0 和 1，应确定正确值后收紧
- [ ] **reflect handler 提取** — 4 个 handler 目前 inline 在 `main.rs`，应提取到独立模块 `src/reflect/`

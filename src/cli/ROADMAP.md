# ROADMAP qtcloud-code-cli

工作清单见 [TODO](./TODO.md)。

## 意图

把实验室的成果迁入正式代码。为先不侵入既有行为，先把确定性分析工具作为新增模块落进 `src/reflect` 并在 `lib.rs` 公开（D1），以薄 example 调用验证；再重构 `main.rs` 的 reflect 子命令改用该模块；最后以测试与 example 判定「功能不变且新增功能」。

三个阶段：

1. 迁 reflect 入 `src`——新增模块并公开，`main.rs` 不动，既有子命令行为不变；
2. 重构 `main.rs`——三个 `run_reflect_*` 改用 `reflect::*`，`graph` 输出重新设计；
3. example 验收——对照重构前后行为，CLI 契约不变即功能不变，AST 追溯与真实调用图等即新增功能，example 保留为活文档。

## 现状与差距

reflect 四个子命令当前在 `main.rs` 里是行号/文本启发式实现，实验室的 AST 版实现尚未进入正式代码。

| 实验室能力 | 生产现状 | 差距 |
|--|--|--|
| `backward_slice` AST 依赖追溯 | 取目标行之前 10 行文本 | 未做依赖追溯 |
| `trace_variable` 数据流 | 文本匹配首个 `let`/`=` | 未做 RHS 上游串联、跨函数不工作 |
| `build_call_graph` 调用图 | 固定输出「调用: 0, 被调用: 0」 | 桩实现 |
| `suggest` 可疑行 | 文本匹配 return/panic 等 | 基本可用，保留 |
| `forward_slice`、`flatten_stmts`、`type_info`、`impact_analysis`、`code_search` | 无 | 缺 |
| `compute_confidence` 证据锚定率 | 无 | 缺 |

实验室的 `llm.rs`（Vault 取密钥 + `enhance_finding`）不迁——生产 `src/llm.rs` 已用环境变量加 OpenAI 兼容接口实现同类能力。

## 阶段一 迁 reflect 入 src

建立 `src/reflect/`，实现自实验室抽取，合并重复的 `walk_all`；`lib.rs` 增加 `pub mod reflect;`。此为新模块，不改 `main.rs`，既有 reflect 子命令行为不变。

目标结构：

```text
src/reflect/
├── mod.rs        SliceEntry / FlowEntry 类型与子模块导出
├── slice.rs      backward_slice / flatten_stmts
├── dataflow.rs   trace_variable
└── analysis.rs   forward_slice / build_call_graph / impact_analysis / code_search / type_info
```

`cross_function_slice` 不迁，并删除实验室对应代码。`compute_confidence` 暂不迁，先在 example 中内联演示，归属层留待语义统一后决定。

example 为薄驱动，直接调用 `qtcloud_code_cli::reflect::*`，覆盖 `slice`、`trace`、`graph`、`suggest` 四个子命令；`chain_exp` 与 `llm_exp` 改写为 example，改用生产 `src/llm.rs` 的环境变量配置，不引入 Vault，未配置 LLM 时跳过。

## 阶段二 重构 main.rs

`run_reflect_slice`、`run_reflect_trace`、`run_reflect_graph` 改为调用 `reflect::*`，参数与退出码保持不变；`graph` 的 JSON 重新设计，同步更新 `tests/reflect.rs`。`trace_variable` 本轮补齐跨函数追踪。`suggest` 保留文本实现。

`graph` 由桩升级为真实调用图、`slice`/`trace` 由文本匹配升级为 AST 追溯，属新增功能而非回归。

## 阶段三 验收

功能不变以 `tests/reflect.rs` 全绿为准，重构前行为对照依赖 git 历史中的既有实现；新增功能以真实调用图、跨函数追踪、`forward_slice`、`type_info`、`impact_analysis`、`code_search` 为准。

```sh
cargo build --examples
cargo test
cargo run -- review .
cargo run -- audit .
```

## 收尾

同步 README、AGENTS.md、CHANGELOG 与 ROADMAP，登记本轮未完成项。`apps/qtcloud-code` 子模块提交并推送，再更新父仓库指针。

## 其他事项

- `ListRules` 与 `contract list` 重复 — 删掉 `ListRules`，统一走 `contract list`；当前 patch 先废弃（`#[deprecated]` 提示走 `contract list`），下一个 minor 移除；
- `build_call_graph` 过滤第三方库调用，使调用数不再虚高；
- Review 验证闭环：修改后重新 review，自动对比前后 finding；
- refactor 提取函数：依赖 LLM 生成代码，需人工审核。

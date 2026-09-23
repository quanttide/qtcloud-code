# ROADMAP qtcloud-code-cli

工作清单见 [TODO](./TODO.md)，决策结论见 [AGENTS](./AGENTS.md)。

## 意图

把实验室的成果迁入正式代码。为暂不改动既有行为，先把确定性分析工具作为新增模块落进 `src/reflect` 并在 `lib.rs` 公开，以薄 example 调用验证；再重构 `main.rs` 的 reflect 子命令改用该模块；最后以测试与 example 判定「功能不变且新增功能」。

三个阶段：

1. 迁 reflect 入 `src`——新增模块并公开，`main.rs` 仅做行为保持的提取（已完成）；
2. 重构 `main.rs`——三个 `run_reflect_*` 改用 `reflect::*`，`graph` 按新契约输出；
3. 验收——以测试与 example 判定验收口径，example 保留为活文档。

验收口径（D8）：功能不变 = CLI 契约不变，即 `slice`、`trace`、`suggest` 的参数、退出码与 JSON 结构保持，`suggest` 输出一致；`slice`、`trace` 由文本启发式升级为 AST 追溯，输出内容按新增功能验收，不算回归。

## 现状与差距

reflect 四个子命令当前在 `main.rs` 里是行号/文本启发式实现，实验室的 AST 版实现尚未进入正式代码。

| 实验室能力 | 生产现状 | 差距 |
|:--|:--|:--|
| `backward_slice` AST 依赖追溯 | 取目标行之前 10 行文本 | 未做依赖追溯 |
| `trace_variable` 数据流 | 文本匹配首个 `let`/`=` | 未做 RHS 上游串联、跨函数不工作 |
| `build_call_graph` 调用图 | 固定输出「调用: 0, 被调用: 0」 | 桩实现 |
| `suggest` 可疑行 | 文本匹配 return/panic 等 | 基本可用，保留 |
| `compute_confidence` 证据锚定率 | 无 | 缺 |

证据计数器（实验室名 `compute_confidence`，example 中定名为 `count_evidence`）按行号/变量名引用计数分级——是证据发现与计数机制而非置信度，与生产 `src/llm.rs` 已有 `confidence` 字段（LLM 自评 confirm/dismiss）是两回事，归属层待 example 演示后决定。

实验室的 `llm.rs`（Vault 取密钥 + `enhance_finding`）不迁——生产 `src/llm.rs` 已用环境变量加 OpenAI 兼容接口实现同类能力。

## 阶段一 迁 reflect 入 src

阶段一已完成：`src/reflect/` 五个模块随 `pub mod reflect`、`pub mod walk`、`pub mod review` 落地；`walk_all` 四份收敛为 `walk` 模块，`refactor/rename.rs` 一并改用（D14）；`main.rs` 完成行为保持的提取——`suggest` 迁入 `reflect::suggest`，review 扫描管线迁入 `review` 模块，既有子命令行为不变（D12）；`src/llm.rs` 公开 `pub fn call_llm` 与取 key 函数，`review` 模块公开 findings 收集管线；七个 example 建成且 `cargo build --examples` 通过；实验室 `chain_exp`、`llm_exp`、`lab.rs` 原件已删（D13）。当前结构：

```text
src/reflect/
├── mod.rs        SliceEntry / FlowEntry / Suggestion 类型与子模块导出
├── slice.rs      backward_slice / flatten_stmts
├── dataflow.rs   trace_variable
├── analysis.rs   forward_slice / build_call_graph / impact_analysis / code_search / type_info
└── suggest.rs    suggest（自 main.rs 迁入的文本实现）
```

`compute_confidence` 暂不迁，example 中已按证据发现与计数重定名为 `count_evidence` 内联演示，归属层待定。剩余两项：

- 为 `analysis.rs` 补单元测试（D11）——四项新增能力的验收与覆盖率都落在单测上；
- 删除实验室 `cross_function_slice` 及其专用辅助函数。

## 阶段二 重构 main.rs

先起草 `graph` 新 JSON 契约（函数节点加调用边），直接落入 `docs/user-guide/reflect.md`——先写契约，后改测试（D10）。`run_reflect_slice`、`run_reflect_trace`、`run_reflect_graph` 改为调用 `reflect::*`，参数与退出码保持不变；`build_call_graph` 过滤第三方库调用，避免调用数虚高（D15）。

`tests/reflect.rs` 四个子命令的断言全部按新实现重写，对照退回 git 历史人工比对（D9）。`trace_variable` 本轮补齐跨函数追踪，`suggest` 保留文本实现；`ListRules` 加 `#[deprecated]` 指向 `contract list`，下一个 minor 移除（D15）。

`graph` 由桩升级为真实调用图、`slice`、`trace` 由文本匹配升级为 AST 追溯，属新增功能而非回归。

## 阶段三 验收

功能不变以 CLI 契约为准（D8）：`tests/reflect.rs` 全绿，`slice`、`trace`、`suggest` 的参数、退出码与 JSON 结构同既有实现，`suggest` 输出一致；重构前行为对照以 git 历史中的既有实现为准，不留额外快照（D9）。`graph` 按新 JSON 契约验收（D10）；新增功能以真实调用图、跨函数追踪与 `analysis.rs` 四项能力的单元测试为准（D11）。

```sh
cargo build --examples
cargo test
cargo llvm-cov
cargo run -- review .
cargo run -- audit .
```

## 收尾

同步 README、AGENTS.md、CHANGEMAP 与 ROADMAP；`docs/user-guide/reflect.md` 与 `docs/dev-guide/reflect-integration-tests.md` 必须同步，`docs/dev-guide/reflect.md` 按实现变更幅度决定（D16）。登记本轮未完成项。`apps/qtcloud-code` 子模块提交并推送，再更新父仓库指针。

## 下轮 backlog

`ListRules` 废弃与 `build_call_graph` 过滤第三方调用已并入本轮（D15），以下两项登记下轮：

- Review 验证闭环：修改后重新 review，自动对比前后 finding；
- refactor 提取函数：依赖 LLM 生成代码，需人工审核。

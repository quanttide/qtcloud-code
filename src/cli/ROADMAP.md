# ROADMAP qtcloud-code-cli

工作清单见 [TODO](./TODO.md)，决策记录见 [DECISIONS](./DECISIONS.md)。

## 意图

把实验室的成果迁入正式代码。为暂不改动既有行为，先把确定性分析工具作为新增模块落进 `src/reflect` 并在 `lib.rs` 公开，以薄 example 调用验证；再重构 `main.rs` 的 reflect 子命令改用该模块；最后以测试与 example 判定「功能不变且新增功能」。

三个阶段：

1. 迁 reflect 入 `src`——新增模块并公开，`main.rs` 不动，既有子命令行为不变；
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
| `forward_slice`、`flatten_stmts`、`type_info`、`impact_analysis`、`code_search` | 无 | 缺 |
| `compute_confidence` 证据锚定率 | 无 | 缺 |

`compute_confidence` 按行号/变量名引用计数分级，与生产 `src/llm.rs` 已有 `confidence` 字段（LLM 自评 confirm/dismiss）语义不同，归属层待 example 演示后决定。

实验室的 `llm.rs`（Vault 取密钥 + `enhance_finding`）不迁——生产 `src/llm.rs` 已用环境变量加 OpenAI 兼容接口实现同类能力。

## 阶段一 迁 reflect 入 src

建立 `src/reflect/`，实现自实验室抽取；`walk_all` 合并实验室三处与 `refactor/rename.rs` 共四份，收敛为公开的公共模块，`refactor/rename.rs` 一并改用（D14）。`lib.rs` 增加 `pub mod reflect;`。此为新模块，不改 `main.rs`，既有 reflect 子命令行为不变。

目标结构：

```text
src/reflect/
├── mod.rs        SliceEntry / FlowEntry 类型与子模块导出
├── slice.rs      backward_slice / flatten_stmts
├── dataflow.rs   trace_variable
└── analysis.rs   forward_slice / build_call_graph / impact_analysis / code_search / type_info
```

`CallGraphNode`、`ImpactResult`、`TypeInfo` 随 `analysis.rs` 定义并 re-export（D14）。`cross_function_slice` 不迁并删除实验室对应代码；`chain_exp`、`llm_exp` 改写为 example 后原件一并删除，`lab.rs` 无引用则删（D13）。`compute_confidence` 暂不迁，先在 example 中内联演示，归属层留待语义统一后决定。

为 `analysis.rs` 补单元测试（D11）——可迁的实验室测试只有 slice 8 个、dataflow 4 个，覆盖率与四项新增能力的验收都落在单测上。`src/llm.rs` 暴露 `pub fn call_llm`、取 key 函数与 review runner（findings 收集），example 一步获得 findings 并做自由 prompt 调用（D12）。

example 为薄驱动，直接调用 `qtcloud_code_cli::reflect::*`，覆盖 `slice`、`trace`、`graph`、`suggest` 四个子命令；`chain_exp` 与 `llm_exp` 改写为 example，改用生产 `src/llm.rs` 的环境变量配置，不引入 Vault，findings 经 lib 暴露的 review runner 产生，未配置 LLM 时跳过。

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

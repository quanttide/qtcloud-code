# TODO 实验室成果迁移

步骤说明见 [ROADMAP](./ROADMAP.md)，决策记录见 [DECISIONS](./DECISIONS.md)。

## 阶段一 迁 reflect 入 src

- [ ] 建 `src/reflect/mod.rs`：`SliceEntry` / `FlowEntry` 并导出子模块
- [ ] 合并实验室三处与 `refactor/rename.rs` 共四份 `walk_all` 为公开公共模块，`refactor/rename.rs` 改用（D14）
- [ ] 建 `src/reflect/slice.rs`：`backward_slice`、`flatten_stmts`
- [ ] 建 `src/reflect/dataflow.rs`：`trace_variable`
- [ ] 建 `src/reflect/analysis.rs`：`forward_slice`、`build_call_graph`、`impact_analysis`、`code_search`、`type_info`，类型在此定义并 re-export（D14）
- [ ] `src/lib.rs` 增加 `pub mod reflect;`
- [ ] 迁实验室已有单元测试（slice 8 个、dataflow 4 个）
- [ ] 为 `analysis.rs` 补单元测试（D11）
- [ ] `src/llm.rs` 暴露 `pub fn call_llm`、取 key 函数与 review runner（findings 收集）（D12）
- [ ] 删除实验室 `cross_function_slice` 及其专用辅助函数
- [ ] 删除实验室 `chain_exp`、`llm_exp`，`lab.rs` 无引用则删（D13）
- [ ] `examples/reflect_slice.rs`、`reflect_trace.rs`、`reflect_graph.rs`、`reflect_suggest.rs`：薄驱动，调用 `qtcloud_code_cli::reflect::*`
- [ ] `examples/evidence_chain.rs`：由 `chain_exp` 改写，正向链与反向链对照，走 `src/llm.rs` 环境变量
- [ ] `examples/false_positive_filter.rs`：由 `llm_exp` 改写，findings 经 lib 暴露的 review runner 产生，未配置 LLM 时跳过
- [ ] `examples/confidence.rs`：内联演示 `compute_confidence`
- [ ] `cargo build --examples` 通过

## 阶段二 重构 main.rs

- [ ] 起草 `graph` 新 JSON 契约（函数节点加调用边），直接落入 `docs/user-guide/reflect.md`，先写契约后改测试
- [ ] `build_call_graph` 过滤第三方库调用（D15）
- [ ] 以 git 历史中的既有实现为对照基准，不额外留存快照
- [ ] `run_reflect_slice` 改调 `reflect::backward_slice`，参数、退出码与 JSON 结构保持
- [ ] `run_reflect_trace` 改调 `reflect::trace_variable`，补齐跨函数追踪，输出按新增功能验收（D8）
- [ ] `run_reflect_graph` 改调 `reflect::build_call_graph`，按新 JSON 契约输出
- [ ] 四个子命令的断言全部按新实现重写，对照退回 git 历史（D9）
- [ ] `run_reflect_suggest` 保留文本实现
- [ ] `ListRules` 加 `#[deprecated]` 指向 `contract list`（D15）

## 阶段三 验收

- [ ] `cargo test` 全绿
- [ ] `cargo build --examples` 通过
- [ ] CLI 契约验收：`slice` / `trace` / `suggest` 参数、退出码与 JSON 结构同既有实现，`suggest` 输出一致（D8）
- [ ] `graph` 按新 JSON 契约验收（D10）
- [ ] 新增能力以单测验收：跨函数 `trace`、`forward_slice`、`type_info`、`impact_analysis`、`code_search`（D11）
- [ ] `cargo run -- review .` 与 `cargo run -- audit .` 自举不退化
- [ ] `cargo llvm-cov` 覆盖率不低于现基准（AGENTS.md 记 92%）

## 收尾

- [ ] 依据 example 表现决定 `compute_confidence` 归属层
- [ ] README 补充新增 reflect 能力
- [ ] AGENTS.md 更新模块结构
- [ ] 同步 `docs/user-guide/reflect.md` 与 `docs/dev-guide/reflect-integration-tests.md`，`dev-guide/reflect.md` 视变更幅度（D16）
- [ ] CHANGELOG 增加条目
- [ ] ROADMAP 登记本轮未完成项
- [ ] `apps/qtcloud-code` 子模块提交并推送，再更新父仓库指针

# TODO 实验室成果迁移

步骤说明见 [ROADMAP](./ROADMAP.md)。

## 阶段一 迁 reflect 入 src

- [ ] 建 `src/reflect/mod.rs`，定义 `SliceEntry` / `FlowEntry` 并导出子模块
- [ ] 建 `src/reflect/slice.rs`：`backward_slice`、`flatten_stmts`
- [ ] 建 `src/reflect/dataflow.rs`：`trace_variable`
- [ ] 建 `src/reflect/analysis.rs`：`forward_slice`、`build_call_graph`、`impact_analysis`、`code_search`、`type_info`
- [ ] 合并三处重复的 `walk_all`
- [ ] `src/lib.rs` 增加 `pub mod reflect;`
- [ ] 迁实验室已有 `#[cfg(test)]` 单元测试
- [ ] 删除实验室 `cross_function_slice` 及其专用辅助函数
- [ ] `examples/reflect_slice.rs`、`reflect_trace.rs`、`reflect_graph.rs`、`reflect_suggest.rs`：薄驱动，调用 `qtcloud_code_cli::reflect::*`
- [ ] `examples/evidence_chain.rs`：由 `chain_exp` 改写，正向链与反向链对照，走 `src/llm.rs` 环境变量
- [ ] `examples/false_positive_filter.rs`：由 `llm_exp` 改写，未配置 LLM 时跳过
- [ ] `examples/confidence.rs`：内联演示 `compute_confidence`
- [ ] `cargo build --examples` 通过

## 阶段二 重构 main.rs

- [ ] 以 git 历史中的既有实现与 `tests/reflect.rs` 为对照基准，不额外留存快照
- [ ] `run_reflect_slice` 改调 `reflect::backward_slice`，参数、文本输出与退出码保持
- [ ] `run_reflect_trace` 改调 `reflect::trace_variable`，补齐跨函数追踪
- [ ] `run_reflect_graph` 改调 `reflect::build_call_graph`，JSON 重新设计
- [ ] 更新 `tests/reflect.rs` 以匹配新契约
- [ ] `run_reflect_suggest` 保留文本实现

## 阶段三 验收

- [ ] `cargo test` 全绿
- [ ] `cargo build --examples` 通过
- [ ] 对照既有实现：`slice` / `trace` / `suggest` 行为一致（D5）
- [ ] `graph` 按新 JSON 契约验收（D4）
- [ ] `cargo run -- review .` 与 `cargo run -- audit .` 自举不退化
- [ ] `cargo llvm-cov` 覆盖率不低于现基准

## 收尾

- [ ] 依据 example 表现决定 `compute_confidence` 归属层
- [ ] README 补充新增 reflect 能力
- [ ] AGENTS.md 更新模块结构
- [ ] CHANGELOG 增加条目
- [ ] ROADMAP 登记本轮未完成项
- [ ] `apps/qtcloud-code` 子模块提交并推送，再更新父仓库指针

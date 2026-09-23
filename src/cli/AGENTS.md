# Agent 工作指南

本文档为 Agent 在 qtcloud-code-cli 中工作提供指南，贡献流程与测试规范见 [CONTRIBUTING](./CONTRIBUTING.md)，架构与开发知识见 [docs/dev-guide](./docs/dev-guide/index.md)。

## 决策风格

以下由实验室成果迁移的决策推测得出（工作清单见 [TODO](./TODO.md)，背景见 [ROADMAP](./ROADMAP.md)）。决策均已拍板并收入本节，原文见 git 历史中的 DECISIONS。项目处于早期，更新可以激进，不为旧契约、死代码或临时产物留包袱：

- 以原始意图为准——选项与建议不符意图时改选，取「先公开 `pub mod reflect` 再写薄 example」，放弃内联自包含；
- 消除临时产物与死代码——直接删除实验室 `cross_function_slice` 与 `chain_exp`、`llm_exp` 原件，`lab.rs` 无引用则删，不留输出快照，example 不做自包含副本（D13）；
- 消除重复实现——四份 `walk_all` 收敛为公开的公共模块，`refactor/rename.rs` 一并改用，不留副本（D14）；
- 能力做完整而非留待办——补齐 `trace_variable` 跨函数追踪，不接受只标注限制；新增能力必须配单元测试，覆盖率不低于现基准（D11）；
- 不迁就旧契约——重新设计 `graph` 的 JSON，测试断言随新实现全部重写，对照只认 git 历史，不走追加字段与保留旧断言的兼容路线（D9）；
- 验收以 CLI 契约为准——参数、退出码与 JSON 结构不变即功能不变，`slice`、`trace` 输出内容升级按新增功能验收（D8）；
- 契约先落文档后改代码——`graph` 新 JSON 契约直接写入 `docs/user-guide/reflect.md`，收尾同步 user-guide 与集成测试文档（D10、D16）；
- example 只用公开能力——`lib` 暴露 `call_llm`、取 key 函数与 review runner，不在 example 内联 HTTP 与扫描逻辑（D12）；
- 已知缺陷不进验收——`build_call_graph` 过滤第三方库调用并入本轮，依赖 LLM 闭环的事项登记下轮 backlog（D15）；
- 未定则先搁置——`compute_confidence` 先放进 example，待语义统一后再定归属层；
- 遵循既有规则——按仓库「提交即推送」执行，并同步父仓库指针；
- 先文档后代码，逐项拍板，决策留痕。

# TODO 实验室成果迁移

步骤说明见 [ROADMAP](./ROADMAP.md)，决策结论见 [AGENTS](./AGENTS.md)。

## 阶段一 迁 reflect 入 src

- [ ] 为 `analysis.rs` 补单元测试（D11）
- [ ] 删除实验室 `cross_function_slice` 及其专用辅助函数

## 阶段二 重构 main.rs

- [ ] `build_call_graph` callee 归一化：取终末方法短名、剔除闭包体（D15 拆解）
- [ ] 项目内调用过滤复用 `audit::project_refs`，外部/标准库调用按策略处理（D15 拆解）
- [ ] callee 输出单行限长，防长链撑爆输出与 JSON 契约（D15 拆解）
- [ ] 起草 `graph` 新 JSON 契约（函数节点加调用边），语义定型后落入 `docs/user-guide/reflect.md`，契约先于测试改动
- [ ] 多语言节点识别：slice/trace/graph 接线时移植 `main.rs` 的多语言函数定位与声明识别（lab 仅 Rust，py 探针见 dev-guide/reflect.md）
- [ ] `suggest` 词表按真实案例校准：cast 放宽到任意 `as` 类型、增补 unwrap/expect、return 类按分级降权
- [ ] 以 git 历史中的既有实现为对照基准，不额外留存快照
- [ ] `run_reflect_slice` 改调 `reflect::backward_slice`，参数、退出码与 JSON 结构保持
- [ ] `run_reflect_trace` 改调 `reflect::trace_variable`，补齐跨函数追踪，输出按新增功能验收（D8）
- [ ] `run_reflect_graph` 改调 `reflect::build_call_graph`，按新 JSON 契约输出
- [ ] 四个子命令的断言全部按新实现重写，对照退回 git 历史（D9）
- [ ] `ListRules` 加 `#[deprecated]` 指向 `contract list`（D15）

## 阶段三 验收

- [ ] `cargo test` 全绿
- [ ] `cargo build --examples` 通过
- [ ] CLI 契约验收：`slice` / `trace` / `suggest` 参数、退出码与 JSON 结构同既有实现，`suggest` 输出一致（D8）
- [ ] `graph` 按新 JSON 契约验收（D10）
- [ ] 真实案例快照测试：`graph` 输出在 `assets/fixtures/search.rs` 上锁定，防回退
- [ ] 新增能力以单测验收：跨函数 `trace`、`forward_slice`、`type_info`、`impact_analysis`、`code_search`（D11）
- [ ] `cargo run -- review .` 与 `cargo run -- audit .` 自举不退化
- [ ] `cargo llvm-cov` 覆盖率不低于现基准（AGENTS.md 记 92%）

## 收尾

- [ ] 依据 example 表现决定证据计数器（`count_evidence`）归属层
- [ ] README 补充新增 reflect 能力
- [ ] 同步 `docs/user-guide/reflect.md` 与 `docs/dev-guide/reflect-integration-tests.md`，`dev-guide/reflect.md` 视变更幅度（D16）
- [ ] CHANGELOG 增加条目
- [ ] ROADMAP 登记本轮未完成项
- [ ] `apps/qtcloud-code` 子模块提交并推送，再更新父仓库指针

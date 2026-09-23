# DECISIONS 决策清单

登记实验室成果迁移评审后仍需拍板的决策，每项给出背景、选项与建议。决策完成后在状态处记录结论，并回写 [TODO](./TODO.md)；步骤背景见 [ROADMAP](./ROADMAP.md)。

上一轮 D1–D7 已拍板并随 63554da 删除，结论吸收进 [AGENTS](./AGENTS.md) 的决策风格，原文见 git 历史；本轮编号自 D8 续接，避免与 ROADMAP、TODO 中残留的（D1）（D4）（D5）混淆，残留引用按本轮结论清理。

## D8 「功能不变」验收口径

背景：ROADMAP 阶段二判定 slice/trace 由文本匹配升级为 AST 追溯属新增功能，TODO 阶段三却要求 slice/trace/suggest 对照既有实现行为一致；trace 补齐跨函数追踪后输出必然不同，两处标准只能对一个。「功能不变以 tests/reflect.rs 全绿为准」又与阶段二「更新 tests/reflect.rs」构成循环自证。

选项：

1. 功能不变 = CLI 契约不变：参数、退出码与 JSON 顶层结构保持，slice/trace 输出内容按新增功能验收，suggest 输出保持一致；
2. 要求 slice/trace/suggest 输出与既有实现逐字一致，升级后做输出适配。

建议：选 1，与 ROADMAP 意图段「CLI 契约不变即功能不变」一致；选 2 迫使 trace 保留旧输出格式，与本轮升级目标矛盾。

状态：1

## D9 tests/reflect.rs 更新范围

背景：阶段二同时要「以 git 历史中的既有实现与 tests/reflect.rs 为对照基准」和「更新 tests/reflect.rs 以匹配新契约」，不界定范围，基准会在同一批提交里被改掉。

选项：

1. 仅重写 graph 断言以匹配新 JSON 契约，slice/trace/suggest 断言原样保留作回归锚点；
2. 四个子命令的断言全部按新实现重写，对照退回 git 历史人工比对。

建议：选 1，既有 slice/trace/suggest 断言是宽松的内容断言，原样可用；选 2 会失去唯一的机械对照物，阶段三只能人工翻 git 历史。

状态：2

## D10 graph 新 JSON 契约的落点

背景：上一轮只拍了「重新设计」，新契约的字段与结构从未写入存留文档，DECISIONS 删除后无处可查；tests 现断言顶层数组与 `name` 字段。契约不先落地，阶段二改不了测试，阶段三无从验收。

选项：

1. 契约草案先登记进本文件，拍板后落入 `docs/user-guide/reflect.md`；
2. 直接在 `docs/user-guide/reflect.md` 写契约，本文件只登记「先写契约后改代码」；
3. 实现时边写边定，验收以最终实现为准。

建议：2

状态：待拍板。

## D11 analysis.rs 测试与覆盖率验收

背景：实验室 `analysis.rs`（`forward_slice`、`build_call_graph`、`impact_analysis`、`code_search`、`type_info`）没有任何单元测试，可迁的只有 slice 8 个、dataflow 4 个；`compute_confidence` 的 3 个单测随不迁的实验室 `llm.rs` 留在原地。阶段三却要求覆盖率不低于现基准（AGENTS.md 记 92%），并把其中四项列为新增功能验收准，任务清单里没有对应验收手段。

选项：

1. 阶段一新增「为 `analysis.rs` 补单元测试」，四项新增能力以单测作验收载体；
2. 新增能力改由 example 演示验收，覆盖率口径放宽为仅约束既有代码；
3. 四项本轮不验收，从阶段三验收准中移除并登记后续待办。

建议：选 1，与「能力做完整而非留待办」一致；选 2 的 example 不计入 `cargo llvm-cov` 默认统计，覆盖率仍会下降；选 3 与 ROADMAP 把四项列入验收准矛盾。

状态：1

## D12 example 所需的 LLM 与 review 公开 API

背景：`evidence_chain` 需要自由 prompt 的单次 LLM 调用，`false_positive_filter` 需要先在项目上跑 review 得到 findings；生产 `src/llm.rs` 的 `call_llm` 与取 key 逻辑是私有的，review 扫描管线（`run_review`/`scan_file`/`create_detectors`）全在 `main.rs`，lib 没有公开 runner；实验室 `llm_exp` 还硬编码了指向其他仓库的绝对路径。

选项：

1. lib 暴露最小 API：`pub fn call_llm` 与取 key 函数，example 自行组装 detector 加 `walk_tree` 产生 findings；
2. 再暴露一个 review runner，example 一步获得 findings；
3. example 内联 HTTP 调用与扫描逻辑，不改 `src`。

建议：选 1，`call_llm` 提为 pub 是小改动，detector 与 `walk_tree` 已公开，组装约十几行；选 2 把 `main.rs` 管线抽象进 lib，扩大本轮范围；选 3 违背薄 example 初衷且制造重复实现。

状态：2

## D13 实验室遗留代码去留

背景：`cross_function_slice` 已定删除；`chain_exp` 与 `llm_exp` 改写为 qtcloud-code 的 example 后，实验室 `src/bin` 下的原件及其依赖的 `lab.rs` 去留没有登记，留着就是两份会各自漂移的实现。

选项：

1. 本轮一并删除实验室 `chain_exp`、`llm_exp`，`lab.rs` 看剩余引用，无引用则删；
2. 实验室保留原件作对照，只迁不删；
3. 只删 `cross_function_slice`，其余实验室代码下轮再清。

建议：选 1，遵循「减法优先」与「消除临时产物与死代码」；选 2 留下必然漂移的副本；选 3 的原件与新 example 重复，与本轮「改写为 example」的口径不齐。

状态：1

## D14 walk_all 合并位置与类型归属

背景：「合并三处重复的 `walk_all`」只指实验室 slice/analysis/dataflow，生产 `src/refactor/rename.rs` 还有第四份，本轮是否覆盖、合并后放哪里都没写；结构图只列 `SliceEntry`/`FlowEntry`，analysis 的 `CallGraphNode`、`ImpactResult`、`TypeInfo` 归属未体现。

选项：

1. 合并为 reflect 模块私有辅助放 `mod.rs`，refactor 副本本轮不动，文档写明只合并迁入的三处；类型随 `analysis.rs` 定义并 re-export，补进结构图；
2. 放公共模块并公开，`refactor/rename.rs` 一并改用，一次消除四份；
3. 不合并，各文件保留自己的实现。

建议：选 1，阶段一约束是不侵入既有行为；选 2 更彻底但动了本轮范围外的代码，可登记其他事项。

状态：2

## D15 其他事项的轮次归属

背景：ROADMAP 其他事项四项不在 TODO 工作清单。`ListRules` 加 `#[deprecated]` 是本轮就要动 `main.rs` 的改动；`build_call_graph` 过滤第三方库调用决定阶段二 graph 验收时调用数是否真实；review 验证闭环与 refactor 提取函数依赖 LLM 能力。

选项：

1. `build_call_graph` 过滤并入阶段二，`ListRules` 废弃单列任务，其余两项登记下轮 backlog；
2. 其他事项整体标记为本轮不做，维持现状验收；
3. 四项全部并入本轮。

建议：选 1，不过滤则虚高的调用数直接进入新契约验收，等于固化已知缺陷；选 3 的后两项依赖 LLM 闭环，明显超出迁移范围。

状态：1

## D16 收尾的文档范围

背景：`docs/user-guide/reflect.md` 记录 graph 等子命令，`docs/dev-guide/reflect.md` 与 `docs/dev-guide/reflect-integration-tests.md` 记录实现与测试；graph JSON 重设计与 trace 跨函数化后这些文档必然过期，而 ROADMAP 与 TODO 的收尾只列 README、AGENTS.md、CHANGELOG、ROADMAP。

选项：

1. 收尾加入 `docs/user-guide/reflect.md` 与 `docs/dev-guide/reflect-integration-tests.md`，`docs/dev-guide/reflect.md` 按实现变更幅度决定；
2. 收尾维持四件套，docs 站点单独一轮维护；
3. docs 三个文件本轮全部同步。

建议：选 1，user-guide 是 D10 契约的落点，不改等于发布过期契约；选 3 的 `dev-guide/reflect.md` 是实现叙述，可等阶段三完成后一次性重写，不必中途同步。

状态：1

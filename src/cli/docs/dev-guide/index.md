# qtcloud-code CLI 架构

## 定位：约束驱动生成

qtcloud-code CLI 是 **AI 编码交付的约束器**——用其他 AI（pi/dsh/Claude Code 等）写代码时，约束 AI 最终交付的**代码、测试、文档三者对齐且质量达标**。

```
约束（契约定义）──→ AI 生成（代码+测试+文档）──→ audit + review 校验
     ↑                                              │
     └──────────── 问题清单反馈 → AI 直接修正 ────────┘
```

## 证据主线（设计总纲）

**证据与发现分层**（对齐家族 `quanttide-audit-toolkit` 四聚合）：证据是**未判定的素材**——独立收集、等待被标准检验；发现是**挂了证据的判定**——由证据匹配标准产生。LLM 只在证据之上解释，从不生产证据。四聚合是家族唯一词汇：

| 框架聚合 | 对应我们的成员 | 说明 |
|:--|:--|:--|
| `AuditCriteria` 标准 | 规则表（`all_rule_ids`/contract）、audit 的期望 | 尺子，独立于素材与判定 |
| `AuditEvidence` 证据 | reflect 的切片/数据流/调用图/类型信息、命中行原文与度量 | 描述性素材，无判定 |
| `AuditFinding` 发现 | review 规则命中、audit 对齐差异 | 问题层——携 criterion、待补 `evidence[]` |
| `AuditReport` 报告 | 问题清单与 findings 的聚合导出 | 边界导出，登记下轮 |

流水线（取 → 排 → 用 → 评）：

```text
取证据  reflect::{backward_slice, trace_variable, build_call_graph, suggest}   已实现
排证据  EvidenceChain：同一组证据的有序组织（正向/反向）             阶段二：src/evidence.rs
用证据  LLM 因果解释：证据链 → prompt → 结论                          example 已通，lib 解释器下轮
评证据  count_evidence → anchored / partial / unanchored               阶段二迁入 evidence 模块
```

落实设计（`src/evidence.rs`，阶段二）：`Evidence` 统一信封（`kind`/`file`/`line`/`text` + 按 kind 的结构化负载）——即 `AuditEvidence` 的结构化形式（框架尚无结构化 location，映射时由信封补齐），reflect 六个输出结构体经 `From` 转入；`EvidenceChain` 为有序证据集 + 来源与目标，graph 的 JSON 契约（D10）取此信封形态；`count_evidence`、`anchor_level` 随迁，归属层就此落定。

边界：证据主线不改变双约束核心，reflect 仍是独立工具——**为问题层的 finding 候选补齐 `evidence[]`** 是它的取证职责，补齐才算合格发现。review/audit 输出按四聚合适配（finding 携 criterion 与 evidence[]，severity 轴 RFC→ISO 在适配时拍板）、findings → 定向取证接线登记下轮；`suggest` 输出是线索（indication），弱判定、不入证据层；LLM 语义 finding 属解释层，标记但不入证据层。设计细节见 [reflect.md](reflect.md) 的证据模型与证据流水线。

## 交付约束体系（双约束核心）

| 层 | 命令 | 校验/职责 | 判定方式 |
|----|------|----------|---------|
| **对齐约束** | `audit` | 代码↔测试↔文档三者对齐 | 机器可判定（提取/对比） |
| **质量约束** | `review` | 代码问题（安全/坏味道/未使用） | 规则引擎 + LLM |

**修复链路**：问题清单（audit/review 输出）→ **AI 直接修正**（不依赖 CLI 修复命令——AI 按清单改，再校验）。

## audit（对齐审计——核心）

AI 交付后校验三角对齐：

| 边 | 校验内容 | 机制 |
|----|---------|------|
| 代码 ↔ 文档 | API 结构一致（函数/签名/参数） | 提取器（AST）↔ 文档声明对比 |
| 代码 ↔ 测试 | 测试引用的 API 存在且签名一致 | 静态分析测试引用 ↔ 代码导出 |
| 测试 ↔ 文档 | 文档声明的行为有测试覆盖 | 文档声明集合 ↔ 测试引用集合 |

输出：**问题清单**（{类型, API, 位置, 期望, 实际}）——清单即 AI 的修正任务。退出码 0/1，可入 CI。详见 [audit.md](audit.md)。

## review（质量约束）

audit 查对齐，review 查质量——两个维度互补，共同构成交付约束。规则引擎兜底 + LLM 语义审查（安全漏洞、并发 bug）。详见 [review.md](review.md)。

## 约束生效方式

1. **生成前**：契约（contract.yaml）定义对齐与质量要求——作为 AI 任务的规格输入
2. **生成后**：audit + review 校验 → 问题清单 → **AI 按清单修正** → 再校验 → 绿交付
3. **CI 门禁**：合并前校验必须绿（约束强制执行）

## 驱动流程（scaffold 生成 + audit 校验闭环）

| 流程 | 起点 | 生成 | 校验 |
|------|------|------|------|
| **文档驱动** | 文档声明 API（docs/api.md） | `scaffold tests <文档>` 生成测试骨架 | audit（代码↔测试↔文档） |
| **测试驱动** | 测试引用 API（tests/*） | `scaffold code <测试>` 生成代码骨架 | audit（测试↔代码） |

audit 红态问题清单即下一步任务：`测试引用不存在` → `scaffold code`；`文档声明无测试覆盖` → `scaffold tests`。

## 设计原则

- **约束先行**：契约是生成任务的规格输入，不是事后检查清单
- **机器可判定优先**：对齐校验不依赖 LLM 判断；质量校验规则引擎兜底
- **反馈可消费**：问题清单结构化——AI 直接按清单修正
- **只读安全**：audit/review 不修改任何文件
- **素材先于判定**：证据是未判定的素材，finding 挂上 evidence 才合格，LLM 只在证据之上解释，语义 finding 属解释层不冒充证据；
- **评证名实相符**：证据计数（`count_evidence`）与 LLM 自评（`confidence`）分名而治，不互相冒充；

## 人机协作模型

3R = review → reflect → refactor。这是人类高级程序员的工作范式总结，不是工具要实现的系统架构。

```text
人类（高级程序员）:
  review:   规则引擎做第一道扫描，辅助发现
  reflect:  根因追溯在我脑子里，工具提供证据
  refactor: 重构决策在我脑子里，工具负责执行

AI（初级程序员）:
  - 规则引擎 = lint 工具，快、确定、无遗漏
  - LLM = 辅助推理，理解语义、提供洞察
```

人类定策略：

```text
--mode lint  仅规则引擎（秒级）
--mode llm   规则引擎 + LLM 审查（分钟级，默认）
--mode deep  规则引擎 + LLM + LLM 修复（需要审核）
```

## 发现分级

遵循 RFC 2119 语义：

| **级别** | **含义** | **举例** |
|:--|:--|:--|
| MUST | 可能引入 bug，必须审查 | unsafe 块 >8 条 |
| SHOULD | 维护负担，建议重构 | 函数 >50 行 |
| MAY | 风格建议，可选采纳 | 函数 >30 行 |

同一规则可输出多个级别，取决于超标程度。例如函数 70 行输出 SHOULD，110 行输出 MUST。

## 审查模式

```text
review --mode lint
  └─ 规则引擎扫描（快、确定）
  └─ 输出 finding

review --mode llm（默认）
  └─ 规则引擎扫描（同 lint）
  └─ LLM 二次审查
       ├── 优先级排序、去重
       ├── 上下文追加
       └── 纯 LLM 规则（安全漏洞、并发 bug 等语义问题）

review --mode deep
  └─ 规则引擎 + LLM 审查（同 llm）
  └─ LLM 生成修复 patch
       └── dry-run 默认，--apply 确认
```

## 规则引擎定位

规则引擎不是主力，是安全网：

- LLM 遗漏了：规则引擎兜底，不放过任何已知模式；
- LLM 误判了：规则引擎给出确定性证据；
- 无 LLM 时：规则引擎独立运行，模式退化为 lint；
- 确定性基线：无论 LLM 版本如何，lint 结果一致。

## 检测器分类

| **类型** | **执行引擎** | **举例** |
|:--|:--|:--|
| 语法规则 | 规则引擎（tree-sitter） | 过长函数、unsafe 块、过长参数列表 |
| 编译规则 | 规则引擎（cargo check） | 未使用变量 |
| 项目规则 | 规则引擎（文件映射） | 缺失测试 |
| 语义规则 | LLM 审查 | 安全漏洞、并发 bug、逻辑错误 |

### 跨语言检测注意事项

不同语言 tree-sitter 节点结构差异大，检测器需处理：

- Rust：`function_item` → `parameters` → `parameter`（每个参数独立节点）；
- Python：`function_definition` → `parameters`（与 Rust 结构兼容）；
- Go：`function_declaration` → `parameters` → `parameter_declaration` → 多个 `identifier`（共享类型声明）；
- Dart：`function_declaration` → `function_signature` → `identifier`（函数名在孙子节点）；
- TypeScript：同 Go 与 Dart 的 `function_declaration` 结构。

优先使用 `child_by_field_name("parameters")`，必须为各语言准备 fallback。

### 配置驱动排除

三层过滤减少检测噪音：

1. 硬编码跳过（`target/`、`.git/`、非源码扩展名）；
2. 启发式判断（inline test、external test file）；
3. 用户配置排除（`.quanttide/code/contract.yaml` 的 `exclude` 字段）。

## 模块结构

```text
src/
├── main.rs          # CLI 入口 (clap)，reflect 子命令暂为文本实现（阶段二接线）
├── lib.rs           # 公开模块
├── config.rs        # .quanttide/code/contract.yaml 配置加载
├── walk.rs          # walk_all 遍历（四份重复实现收敛于此）
├── review.rs        # review 扫描管线（findings 收集，CLI 与 example 共用）
├── llm.rs           # LLM 二次审查；get_api_key / call_llm 公开供 example
├── audit.rs         # 对齐审计（代码↔测试↔文档）
├── contract.rs      # 契约清单与校验
├── scaffold.rs      # 骨架生成
├── output.rs        # 输出格式：JSON / Terminal / STATUS.md
├── parser/          # 语言解析器
│   ├── mod.rs       # LanguageParser trait + ParseResult
│   ├── rust.rs      # RustParser
│   ├── python.rs    # PythonParser
│   ├── go.rs        # GoParser
│   ├── dart.rs      # DartParser
│   └── typescript.rs # TypeScriptParser + TsxParser
├── detector/        # 检测器
│   ├── mod.rs       # Detector trait + Finding + walk_tree
│   ├── long_function.rs
│   ├── long_parameter_list.rs
│   ├── unsafe_block.rs
│   ├── unused_variable.rs
│   └── missing_tests.rs
├── reflect/         # 定向分析
│   ├── mod.rs       # SliceEntry / FlowEntry / Suggestion 类型与导出
│   ├── slice.rs     # backward_slice / flatten_stmts
│   ├── dataflow.rs  # trace_variable
│   ├── analysis.rs  # forward_slice / build_call_graph / impact_analysis / code_search / type_info
│   └── suggest.rs   # suggest（文本启发式，自 main.rs 迁入）
└── refactor/        # 代码变换（rename）
```

`examples/`（薄驱动）与 `assets/fixtures/`（真实案例素材）在 crate 根、`src/` 之外，目录规则见 [../../AGENTS.md](../../AGENTS.md)。阶段二将新增 `src/evidence.rs`（证据信封与评证，见证据主线）。

## 历史与降级工具

- v0.2.x 曾以 3R（review→reflect→refactor）为**核心人机协作范式**——该范式已淘汰；review 保留（质量约束），reflect/refactor 降级为**独立分析工具**（不入交付约束体系）：
  - `reflect`：定向代码分析（slice/trace/graph/suggest）——独立工具，人类偶尔使用，见 [../user-guide/reflect.md](../user-guide/reflect.md)
  - `refactor`：实现仅 rename（设计远大于实现，修复主路径已被 LLM 替代）——已从体系移除

# reflect — 证据生产者（独立定向分析工具）

## 职责与定位

定向代码分析（slice/trace/graph/suggest）——你指定方向（文件+行号/变量），工具产出**确定性证据**。

**定位**：~~交付约束体系的修复链路~~（2026-08 已降级）——**独立定向分析工具**，不入交付约束体系（交付约束核心是 audit + review）。但在证据主线下，reflect 是**定向取证层**：给定 review finding 的位置或任意方向，产出可复核、可复现的分析证据，供人直接读、供 LLM 在其上解释（解释不回流为证据）。

给定 review 的线索（finding 候选），不是停留「这里有问题」，而是反复追问「为什么」，直到拿到源头的语句与值路径——**为 finding 补齐 `evidence[]`**，框架里补齐证据的发现才算合格发现。`suggest` 是唯一带弱判定的输出，身份是线索（indication），不入证据层。

## 证据模型

六个输出结构体统一转入证据信封（`src/evidence.rs`，已落地）：

| 结构体 | 字段 | 来源 |
|:--|:--|:--|
| `SliceEntry` | file, line, text | slice / forward_slice 等 |
| `FlowEntry` | var, from, line | trace |
| `CallGraphNode` | name, line, callees, callers | graph |
| `CodeSuggestion` | line, kind, text | suggest |
| `ImpactResult` | def_line, var_name, forward_usages, callees | impact_analysis |
| `CodeTypeInfo` | var, line, type_annotation | type_info |

**已落地（`src/evidence.rs`）**：

- `CodeEvidence` 统一信封：`kind` + `file` + `line` + `text` + 按 kind 的结构化负载（enum payload），六个结构体经 `From` 转入；
- `CodeEvidenceChain`：有序证据集 + 来源与目标，`reversed()` 出反向链——`examples/evidence.rs` 已改用 lib（原内联 `chain_text` 删除）；
- `count_evidence` / `anchor_level` 已迁入同模块——评证与证据同居，归属层就此落定；
- 对齐家族四聚合：`CodeEvidence` 即 `AuditEvidence` 的结构化形式（补 `kind`/`file`/`line`），家族唯一词汇以 `quanttide-audit-toolkit` 为准。

## 证据流水线（取 → 排 → 用 → 评）

| 步骤 | 职责 | 现在在哪 | 目标与计划 |
|:--|:--|:--|:--|
| 取证据 | 四个子命令产出确定性证据 | `reflect::*`，已接线 CLI | Rust AST + `reflect::lang` 多语言定位 |
| 排证据 | 同一证据集的有序组织（正/反向） | `CodeEvidenceChain`（`src/evidence.rs`） | 已落地，`reversed()` 出反向链 |
| 用证据 | 证据链 → LLM prompt → 结论 | example 内拼 prompt + `llm::call_llm` | lib 解释器登记下轮 |
| 评证据 | 结论文本的证据引用计数分级 | `evidence` 模块（已迁入） | 已落地，分级单测随迁 |

## 程序切片

给定程序中一点（finding 位置），反向找出所有可能影响该点的语句。

```
let a = unsafe_ptr();        // ← 被切片包含
let b = a.offset(8);         // ← 被切片包含
let c = *b;                  // ← slicing criterion（finding 所在行）
let x = 1;                   // ← 不影响结果，不在切片内
println!("{}", x);           // ← 不在切片内
```

用于 reflect：给定一个 unsafe 块、空指针、或任何 finding，反向切片找到所有导致它的代码路径。

## 数据流分析

追踪值的定义→使用路径，回答「这个值从哪来到哪去」。

```
input:  finding 位置 + 涉及的变量
output: 值的完整路径图

parse_user_input()            // 用户输入 →
  → to_raw_ptr()              // 转为裸指针 →
    → buffer.write()          // 写入缓冲区 →
      → unsafe { ... }        // finding 位置
```

路径图可直接作为推理链的证据，不需要 LLM。

## 依赖图分析

在项目模块图上追溯，回答「哪些模块链涉及了这个问题」。

```
finding: data/pointer.rs 的 unsafe 块

反向依赖切片：
  data/pointer.rs
    ← data/buffer.rs（调用指针操作）
      ← service/processor.rs（调用 buffer）
        ← api/handler.rs（调用 processor）

正向依赖切片：
  data/pointer.rs
    → 被 3 个模块直接调用
    → 被 7 个模块间接调用
    → 影响范围：整个 data 层和大部分 service 层
```

## 证据链（原「推理链」）

三种分析结果合并为有序证据流——目标 JSON 形态（信封结构，契约以阶段二 D10 起草为准）：

```json
{
  "source": "src/material/mod.rs",
  "target": { "line": 34, "var": "content" },
  "entries": [
    {
      "kind": "flow",
      "file": "src/material/mod.rs",
      "line": 24,
      "text": "let body = ...",
      "var": "body",
      "from": "text.strip_prefix(\"# \")"
    }
  ]
}
```

→ 人直接读这个证据链；LLM 在其上做因果解释（`llm::call_llm`）；`count_evidence` 对结论评锚定分级。历史设计稿里的 `investigations` + `llm_insight` 聚合即此链的完整形态——解释字段随用证阶段落地。

## 证据链示例

```
finding: data/ 层 3 个 unsafe 块、service/ 层 2 个、api/ 层 1 个

机械侦探输出：
  程序切片：每个 unsafe 块的语句路径 → 都在做裸指针操作
  数据流：指针值都来自 buffer.write() → 绕过了 safe API
  依赖图：三个模块各自独立调用底层指针，没有经过中间层

LLM 因果解释：
  "buffer 层的 safe 批量操作缺失，
   导致两个模块各自手写 unsafe 实现。
   根因是架构层缺乏共享抽象，不是 unsafe 本身。"

行动:
  1. 扩展 buffer 层批量操作接口
  2. 替换三层的裸指针操作
  3. 补充架构评审检查项"
```

## 输出格式

`--json` 输出（本轮按新实现定型，D8 结构保持 + D10 graph 新契约）：

| 子命令 | `--json` 输出 | 实现状态 |
|:--|:--|:--|
| slice | `[{line, text}]`（结构保持；Rust 内容为 AST 依赖链，含尾表达式目标） | 已接线 `reflect::backward_slice` |
| trace | `[{line, var, from}]`（结构保持；Rust 为 AST 数据流，含跨函数追踪） | 已接线 `reflect::trace_variable` |
| graph | `{file, nodes[]}`，节点为 `kind:"graph"` 证据信封（含调用边） | 新契约（D10），定稿见 user-guide/reflect.md |
| suggest | `[{line, kind, text}]`，按风险分级排序 | 词表已校准，文本实现保留 |

`investigations` + `llm_insight` 聚合属用证阶段，随 `CodeEvidenceChain` 的解释字段登记下轮；reflect 当前不接 LLM，LLM 与切片的对照实验见 `examples/evidence.rs`。

## 命令行

```sh
qtcloud-code reflect slice <file> <line> [--json]       # 反向切片
qtcloud-code reflect trace <file> <var> [line] [--json]  # 变量数据流
qtcloud-code reflect graph <file> [--json]               # 函数级调用图
qtcloud-code reflect suggest <file> [--json]             # 可疑行推荐
```

reflect 是独立子命令组；`review --reflect` 一类的集成入口属设计愿景，尚未实现。slice/trace/graph 已接线 `reflect::*`（Rust 走 AST，多语言定位经 `reflect::lang` 移植）；py/go/ts 的完整 AST 节点语义登记下轮。

## 素材与真实案例

被分析素材统一放 `assets/fixtures/`（目录规则见 [../../AGENTS.md](../../AGENTS.md) 的素材与示例目录）。现有三个案例自 qtcloud-work 逐字抽取：`as_material`（五级 `let` 链，供 slice/trace）、`search`（真实调用边，供 graph）、`read_criterion`（return 密集，供 suggest）。

驱动素材优先、缺省回落内嵌，定位用 `env!("CARGO_MANIFEST_DIR")`；slice/trace 的目标行运行时自定位，不写死行号。真实案例是算法与规则的试金石——玩具样例只验证「能跑」，演示与验收以 `assets/fixtures/` 的输出为准（快照测试已落地：`test_graph_snapshot_search_fixture` 锁定 `search.rs` 输出）。

## 已知缺陷与限制

真实案例与 py 探针暴露，按发现顺序登记；处理阶段见 ROADMAP 与 TODO：

- 多语言 AST 节点语义（下轮）：定位层已移植 `reflect::lang`（函数作用域/声明行/行级函数清单），接线后 py/go/ts 探针仍通过；`reflect::*` 的 AST 分析仍仅识别 Rust 节点，非 Rust 走行级路径、graph 非 Rust 无调用边；`refactor::rename` 同为仅 Rust 节点，py/go 静默空结果；
- 声明表作用域语义不一致（下轮批量修复）：`slice::build_decls` 容器合并且外层优先（`or_insert`），`dataflow::collect_all_decls` 平铺后见优先（`insert`），同一影子绑定两模块结论相反；
- 解构绑定漏跟（下轮批量修复）：`let (a, b)` 在 slice 取 pattern 首名、在 dataflow 取整段 pattern 文本（`"(a, b)"` 永不匹配标识符），`type_info` 同取首名——第二个名字失跟；
- `forward_slice` 跨作用域同名误命中（下轮批量修复）：全树按名匹配把他函数同名标识符列为使用点，且无 example/测试覆盖，接线前先补用例。

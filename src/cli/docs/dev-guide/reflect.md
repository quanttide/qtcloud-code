# reflect — 独立定向分析工具

## 职责

定向代码分析（slice/trace/graph/suggest）——你指定方向（文件+行号/变量），工具提供证据链。

**定位**：~~交付约束体系的修复链路~~（2026-08 已降级）——**独立定向分析工具**，不入交付约束体系。交付约束核心是 audit（对齐）+ review（质量）；修复由 AI 按问题清单直接完成，reflect 的根因分析不是必经环节。保留 slice/trace/graph/suggest 供人类偶尔做定向分析（依赖追溯、变量定义链）。

给定 review 的证据（finding），不是停留在"这里有问题"，而是反复追问"为什么"，直到找到源头。

## 核心架构

```
证据（review findings）
  ↓
机械侦探（确定性，规则引擎）
  ├── 程序切片    在函数内反向追溯："这个 unsafe 块怎么来的"
  ├── 数据流分析  追踪值路径："这个裸指针从哪里传过来的"
  └── 依赖图分析  跨文件追溯："哪些模块依赖了这个不安全接口"
  ↓
推理链（证据流）
  ↓
因果解释（LLM，可选）
  └── 在证据链基础上回答"为什么"
```

**机械侦探部分不需要 LLM**，结果完全确定、可复现。LLM 只在最后一步做因果解释。

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

追踪值的定义→使用路径，回答"这个值从哪来到哪去"。

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

在项目模块图上追溯，回答"哪些模块链涉及了这个问题"。

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

## 推理链

三种分析结果合并为统一的证据流：

```
evidence_chain:
  [
    { type: "program-slice",  file, lines,     summary: "语句路径" },
    { type: "data-flow",      path,            summary: "值路径" },
    { type: "dep-slice",      chain,           summary: "调用链" },
  ]

→ 人类可以直接读这个证据链
→ LLM 在这之上做因果解释
```

## 推理链示例

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

`--json` 时四个子命令各输出一个顶层数组（阶段二 graph 按 D10 契约重设计）：

| 子命令 | 数组元素字段 | 实现状态 |
|:--|:--|:--|
| slice | `{file, line, text}` | 文本实现，阶段二换 AST |
| trace | `{var, from, line}` | 同上 |
| graph | `{line, name}` | 桩输出（调用数恒为 0），阶段二重设计 |
| suggest | `{line, kind, text}` | 文本实现，保留 |

证据链聚合格式（`investigations` + `llm_insight`）属设计愿景，尚未实现——reflect 当前不接 LLM，LLM 与切片的对照实验见 `examples/evidence.rs`。

## 命令行

```sh
qtcloud-code reflect slice <file> <line> [--json]       # 反向切片
qtcloud-code reflect trace <file> <var> [line] [--json]  # 变量数据流
qtcloud-code reflect graph <file> [--json]               # 函数级调用图
qtcloud-code reflect suggest <file> [--json]             # 可疑行推荐
```

reflect 是独立子命令组；`review --reflect` 一类的集成入口属设计愿景，尚未实现。当前 slice/trace/graph 仍是 `main.rs` 的文本启发式实现，阶段二接线到 `reflect::*`（见 ROADMAP 阶段二）。

## 素材与真实案例

被分析素材统一放 `assets/fixtures/`（目录规则见 [../../AGENTS.md](../../AGENTS.md) 的素材与示例目录）。现有三个案例自 qtcloud-work 逐字抽取：`as_material`（五级 `let` 链，供 slice/trace）、`search`（真实调用边，供 graph）、`read_criterion`（return 密集，供 suggest）。

驱动素材优先、缺省回落内嵌，定位用 `env!("CARGO_MANIFEST_DIR")`；slice/trace 的目标行运行时自定位，不写死行号。真实案例是算法与规则的试金石——玩具样例只验证「能跑」，演示与验收以 `assets/fixtures/` 的输出为准（快照测试见 ROADMAP 阶段三）。

## 已知缺陷与限制

真实案例与 py 探针暴露，按发现顺序登记；处理阶段见 ROADMAP 与 TODO：

- `graph` callee 语义（阶段二 D15 拆解）：方法链整段、闭包体、外部调用混杂入表，`assets/fixtures/search.rs` 输出可见；拆解为终末短名、剔除闭包体、复用 `audit::project_refs` 过滤、单行限长；
- `suggest` 词表过时（阶段二校准）：cast 仅认 `as f64`/`as i32`（真实转换多为 usize/u64，qtcloud-work 全库漏报）、无 unwrap/expect，五类中四类零命中；return 类在验证器型函数误报偏高（`read_criterion` 单函数 10 条），按发现分级降权；
- 多语言回归风险（阶段二接线门禁）：`reflect::*` 仅识别 Rust 节点（`function_item`/`let_declaration`），当前 CLI 为文本实现故 py 探针（slice/trace/graph）通过，直接接线将回归——须移植 `main.rs` 的多语言函数定位与声明识别；`refactor::rename` 同为仅 Rust 节点，py/go 静默空结果；
- 声明表作用域语义不一致（下轮批量修复）：`slice::build_decls` 容器合并且外层优先（`or_insert`），`dataflow::collect_all_decls` 平铺后见优先（`insert`），同一影子绑定两模块结论相反；
- 解构绑定漏跟（下轮批量修复）：`let (a, b)` 在 slice 取 pattern 首名、在 dataflow 取整段 pattern 文本（`"(a, b)"` 永不匹配标识符），`type_info` 同取首名——第二个名字失跟；
- `forward_slice` 跨作用域同名误命中（下轮批量修复）：全树按名匹配把他函数同名标识符列为使用点，且无 example/测试覆盖，接线前先补用例。

# reflect — 根因层

## 职责

像高级工程师/架构师一样做侦探式根因追溯。

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

```json
{
  "mode": "reflect",
  "investigations": [
    {
      "finding_id": "wide-unsafe@data/pointer.rs:12",
      "evidence_chain": {
        "program_slice": {"length": 15, "lines": "..."},
        "data_flow": {"path": ["parse_input", "to_raw_ptr", "buffer_write"]},
        "dep_slice": {"callers": ["service/processor.rs", "api/handler.rs"], "scope": "3 模块"}
      },
      "llm_insight": "buffer 层的 safe 批量操作缺失，导致两个模块各自手写 unsafe",
      "action": "扩展 buffer 层批量操作接口"
    }
  ]
}
```

## 命令行

```sh
review . --reflect            # review + reflect（机械侦探，无 LLM）
review . --reflect --llm      # review + reflect + LLM 因果解释
```

无 LLM 时 reflect 仍可独立运行——输出结构化证据链，只是没有自然语言解释。

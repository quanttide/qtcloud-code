# reflect — 根因层

## 职责

像侦探一样追溯根因。给定 review 的证据（finding），反复追问"为什么"，直到找到源头。

## 与 review 的区别

```
review:   发现"有什么问题"
          "这个 unsafe 块有 9 条语句"

reflect:  追问"为什么会这样"
          "因为三个模块各自实现了同样的指针操作，
           应该统一抽象——这不是 unsafe 的问题，
           是缺乏共享工具函数的问题"
```

## 工作方式

LLM 拿到全部 finding 后，不是聚合统计，而是做侦探推理：

```
证据收集
  ↓
根因追溯（追问 3~5 轮 why）
  ↓
推理链输出（让人类看到思考过程）
  ↓
行动计划（不是 patch，是指出"从哪改起"）
```

## 推理链示例

```
finding: data/ 层 3 个 unsafe 块、service/ 层 2 个、api/ 层 1 个

why: 都在做裸指针操作
  why: 因为没有一个安全的指针抽象
    why: 因为引入外部库需要评审，团队选择自己写
      结论: 根因是缺乏共享工具库，不是 unsafe 本身

行动:
  1. 统一封装 SafePointer 抽象（data 层负责人）
  2. 替换三层的裸指针操作（估计 2 人日）
  3. 建立共享工具库评审流程（长期）
```

## 输出格式

```json
{
  "mode": "reflect",
  "investigations": [
    {
      "evidence": ["data/pointer.rs:12", "data/pointer.rs:45", "service/buffer.rs:33"],
      "finding_ids": ["wide-unsafe@data/pointer.rs:12", "wide-unsafe@data/pointer.rs:45", "wide-unsafe@service/buffer.rs:33"],
      "root_cause": "缺乏共享的安全指针抽象",
      "reasoning_chain": [
        "三个模块各自实现了裸指针操作",
        "因为没有一个统一的 safe 封装",
        "因为团队选择内实现而非引入外部库",
      ],
      "action": "提取 SafePointer 抽象，替换三处实现"
    }
  ]
}
```

## 命令行

```sh
review . --reflect           # review + reflect（根因分析）
review . --reflect --llm     # 同上（当前必须，reflect 纯 LLM）
```

reflect 无 lint 模式——没有 LLM 时 reflect 不运行。

# reflect — 理解层

## 职责

对 review 输出的所有 finding 做全局元分析，生成项目级健康报告。纯 LLM 驱动，无规则引擎参与。

## 与 review 的区别

```
review:  每个 finding 是孤立的
         只看"这段代码有什么问题"

reflect: 把全部 finding 放到一起看
         看"这个项目整体健康度如何"
```

## 执行流程

```
review 输出（全部 finding）
  ↓
LLM 跨 finding 分析
  ├── 按类别聚合（安全、性能、可维护性）
  ├── 按模块分布
  ├── 趋势判断（新增 vs 存量）
  └── 根因追溯
  ↓
reflect 输出（项目级报告）
```

## LLM 分析维度

### 聚合

| 维度 | 做法 |
|------|------|
| 类别 | 安全类、性能类、可维护性类各多少 |
| 分布 | 哪个模块问题最多、哪个类别最集中 |
| 运营 | 新增 vs 存量、重复出现同一模式 |

### 根因

LLM 跨文件寻找模式：
```
"新人在 controller/、service/、repository/ 三层
都写了同样的 unsafe 指针模式——建议统一封装一个 safe 抽象层"
```

### 优先级

在 review 的单个 finding 优先级之上，给出全局排序：

```
P0  内存安全（3 个 MUST，集中在 data 层）
P1  重复代码（5 个模块各有 60+ 行函数）
P2  缺失测试（按模块重要性排序）
```

## 输出格式

```json
{
  "mode": "reflect",
  "summary": {
    "total": 47,
    "must": 3,
    "should": 15,
    "may": 29,
    "semantic": 2
  },
  "by_category": {
    "security": {"count": 3, "priority": "P0"},
    "maintainability": {"count": 25, "priority": "P1"},
    "correctness": {"count": 5, "priority": "P2"}
  },
  "by_module": [
    {"path": "src/data/", "findings": 12, "top_issue": "unsafe 模式重复"},
    {"path": "src/api/", "findings": 8, "top_issue": "过长函数"}
  ],
  "trend": {
    "new_this_cycle": 5,
    "resolved": 12,
    "carried_over": 30
  },
  "recommendations": [
    "P0 优先处理 data 层的 3 个 unsafe——统一封装 SafePointer 抽象",
    "P1 在 controller/service/repository 三层提取共享工具函数",
    "P2 为 data 层模块补充单元测试"
  ]
}
```

## 命令行

```sh
review . --reflect              # review + reflect
review . --reflect --llm        # 同上（当前必须，reflect 纯 LLM 驱动）
review . --mode deep            # review + reflect + refactor
```

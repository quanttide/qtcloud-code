# 3R 架构

## 人机协作模型

```
AI   = 海量初级程序员（LLM 主力干活，规则引擎是工具）
人类 = 高级程序员（定策略、审结果、做判断）
```

## 三层递进

```
review ──→ reflect ──→ refactor
   │           │           │
   │           │           └─ patch（代码修改）
   │           └─ 理解层（LLM 分析 + 解释）
   └─ 检测层（规则引擎 + LLM 审查）
```

每层依赖下一层。上层可以跳过，下层不能跳过。

| 层 | 执行者 | 输出 | 耗时 | 人类介入 |
|----|--------|------|------|----------|
| **review** | 规则引擎 → LLM | finding | 秒~分 | 配置规则、排除误报 |
| **reflect** | LLM | 分析报告 | 分~十秒 | 阅读、做判断 |
| **refactor** | LLM | patch | 十秒~分 | 审核、确认 apply |

## 三种模式映射

```
-review --mode lint  = review（仅规则引擎）
-review --mode llm   = review（规则引擎 + LLM）
-review --mode deep  = review + reflect + refactor（LLM 全流程）
```

## 调用链

```
review:
  1. 规则引擎扫描（tree-sitter / cargo check）
  2. [LLM] 二次审查（排序、去重、语义规则）

reflect:
  1. [LLM] 跨 finding 元分析
  2. [LLM] 生成项目级报告

refactor:
  1. [LLM] 对每个 finding 生成 patch
  2. 人类审核（默认 dry-run）
  3. [--apply] 写入文件
  4. 自动验证（编译 + 测试）
```

## 安全设计

| 层 | 安全保障 |
|----|----------|
| review | 规则引擎兜底，LLM 不产生也不屏蔽 finding |
| reflect | 只读分析，不改代码 |
| refactor | dry-run 默认，apply 需确认，验证不通过回退 |

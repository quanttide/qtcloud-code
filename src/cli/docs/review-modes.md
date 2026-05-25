# review 工作模式

## 人机协作模型

```
AI   = 海量初级程序员（LLM 主力干活）
人类 = 高级程序员（定策略、审结果、做判断）
```

## 三种模式

```
--mode lint  仅规则引擎                   秒级，确定
--mode llm   规则引擎 + LLM 审查         分钟级（默认）
--mode deep  规则引擎 + LLM 审查 + 修复 需审核 patch
```

### lint 模式

纯规则引擎。输入源码 → 输出 finding。完全确定、可复现、无 LLM。

### llm 模式（默认）

```
1. 规则引擎扫描（同 lint）
2. LLM 二次审查：
   ├── 优先级排序、去重
   ├── 上下文追加（代码片段、影响分析）
   ├── 误报标记
   └── 纯 LLM 规则（安全漏洞、并发 bug、逻辑错误）
```

规则引擎是安全网，LLM 是干活的主力。

规则引擎 findings 传递给 LLM 作为上下文，LLM 在此基础上：
- 确认或降级每个 finding
- 追加理解层（"这个函数长是因为验证和逻辑混在一起"）
- 发现规则引擎无法检测的语义问题

### deep 模式

在 llm 基础上增加修复能力：

```
llm 审查通过后 → LLM 生成 patch
                  ├── 默认 dry-run，只显示 diff
                  └── --apply 确认写入
```

安全约束：
- 每个 finding 对应一个 patch，可单独 apply/跳过
- apply 后自动验证（编译 + 测试通过）
- 验证不通过 → 回退并提示

## 命令行

```
qtcloud-code review .                    # 默认 llm 模式
qtcloud-code review . --mode lint        # 仅规则引擎
qtcloud-code review . --mode deep        # 审查 + 修复
qtcloud-code review . --mode deep --apply # 审查 + 修复 + 写入
```

## LLM 调用策略

| 维度 | 策略 |
|------|------|
| 每次调用文件数 | 按文件分批，每批 ≤10 个 finding（上下文窗口） |
| 重试 | 失败重试 1 次，仍失败则降级到 lint 模式对应结果 |
| 成本控制 | 只对规则引擎有 finding 的文件发起 LLM 调用 |
| 缓存 | 同一文件同一版本的 LLM 结果缓存 24h |

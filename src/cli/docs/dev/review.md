# review — 质量校验层

## 职责

找出代码中所有可疑点。不判断、不解释、不修改。

**定位**：交付约束体系的质量维度——audit 查**对齐**（代码↔测试↔文档），review 查**质量**（代码问题）。规则引擎兜底 + LLM 语义审查，共同约束 AI 交付。

## 执行流程

```
源码文件
  ↓
规则引擎扫描 ──→ 语法规则（tree-sitter）
           ├── 编译规则（cargo check）
           └── 项目规则（文件映射）
  ↓
finding 集合
  ↓
[LLM 二次审查]
  ├── 优先级排序
  ├── 去重（合并同类 finding）
  ├── 误报标记（不删除，仅标记）
  └── 语义规则（安全漏洞、并发 bug、逻辑错误）
  ↓
review 输出
```

## 规则引擎

确定性扫描，无 LLM 时独立运行。所有规则在 `detector/` 中定义。

| 规则类型 | 引擎 | 语言 |
|----------|------|------|
| 过长函数 | tree-sitter | 全语言 |
| 过长参数列表 | tree-sitter | 全语言 |
| unsafe 块 | tree-sitter | Rust |
| 未使用变量 | cargo check | Rust |
| 缺失测试 | 文件映射 | 全语言 |

## LLM 二次审查

规则引擎的 finding 作为 LLM 输入。LLM 不产生新 finding 类型（语义规则除外），只对已有 finding 做增强。

### LLM 输入
```
项目语言、框架
每个 finding：位置、规则 ID、严重级别、代码片段
文件完整源码（对有 finding 的文件）
```

### LLM 输出
```
增强后的 finding：
  - 原始信息不变
  - 追加：优先级（高/中/低）
  - 追加：LLM 解释
  - 标记：confirm / dismiss
新增语义 finding：
  - 规则引擎无法检测的问题
  - 说明：所属类别 + 证据
```

## 输出格式

```json
{
  "mode": "review",
  "engine": {"runtime_ms": 1230, "findings": 12},
  "llm": {"runtime_ms": 4500, "findings": 10, "semantic": 2},
  "findings": [
    {
      "file": "src/main.rs",
      "line": 53,
      "severity": "MUST",
      "rule_id": "long-function",
      "message": "函数 `run_review` 共 90 行",
      "llm": {
        "priority": "high",
        "explanation": "前 40 行做路径解析和配置加载，后 50 行做文件扫描——拆成 resolve_config() 和 scan_files() 可降至 45 行",
        "confidence": "confirm"
      }
    }
  ]
}
```

## 命令行

```sh
review .                  # 规则引擎 + LLM（默认）
review . --mode lint      # 仅规则引擎
review . --format json    # JSON 输出
review . --rules long-function,missing-tests  # 仅指定规则
```

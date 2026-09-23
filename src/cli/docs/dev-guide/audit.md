# audit — 对齐审计层

## 职责

**约束 AI 交付物**：校验 AI 生成的代码、测试、文档三者对齐（约束驱动生成——audit 是生成约束的满足度校验）。

```
AI 交付（代码 + 测试 + 文档）
        ↓
qtcloud-code audit --contract contract.yaml
        ↓
三角对齐校验
  ├── 代码 ↔ 文档：API 结构一致（函数/签名/参数）
  ├── 代码 ↔ 测试：测试引用的 API 存在且签名一致
  └── 测试 ↔ 文档：文档声明的行为有测试覆盖
        ↓
绿 ✅（对齐）          红 ❌（问题清单 → AI 按清单修正 → 再 audit）
```

## 在体系中的位置

```text
问题层（并列双轴，门禁有先后）
  audit（对齐轴：代码↔测试↔文档）     review（质量轴：规则引擎 + LLM）
   └─ 问题清单                          └─ findings
          └───────── file + line ─────────┘
                      ▼ 定向取证接口
证据层     reflect——只举证，不提问 → CodeEvidence / CodeEvidenceChain（阶段二）
```

audit 与 review 并列出问题、维度不同（对齐 vs 质量），互不替代——audit 不是被拆进 review/reflect；`audit → review` 只是门禁次序。reflect/refactor 已降级或移除（见 [index.md](index.md) 历史与降级工具），不构成流水线环节。

- 分工：audit 查对齐（代码↔测试↔文档的一致性，机器可判定）；review 查质量（代码可疑点，规则引擎 + LLM）
- 下层不能跳过：audit 不过不进 review

## 与证据主线的关系

对齐差异是**问题层的 finding**——criterion 是期望与规则，evidence 是实际值与声明的原文及提取结果（机判、零 LLM 判断）；与 review 的 findings 同层，finding 携 criterion、待补 `evidence[]`，输出按四聚合适配登记下轮（见 [index.md](index.md) 证据主线）。

## 校验规则（三边）

### 边 1：代码 ↔ 文档

| 规则 | 说明 |
|------|------|
| API 完整 | 代码导出的每个 API 文档必须声明（反之亦然） |
| 签名一致 | 文档声明的参数与代码实际签名一致 |
| 机制 | 提取器（AST）从代码提取结构 ↔ 解析文档声明——对比 |

### 边 2：代码 ↔ 测试

| 规则 | 说明 |
|------|------|
| 引用存在 | 测试引用的 API 必须存在于代码 |
| 签名一致 | 测试调用参数与代码签名一致 |
| 机制 | 静态分析测试文件的 API 引用 ↔ 代码导出对比 |

### 边 3：测试 ↔ 文档

| 规则 | 说明 |
|------|------|
| 行为覆盖 | 文档声明的每个 API 在测试中出现（有测试验证） |
| 机制 | 文档声明的 API 集合 ↔ 测试引用的 API 集合对比 |

## 输出（问题清单）

```
每条问题结构化：{类型, API, 位置, 期望, 实际}
例：
  ✗ 代码有文档无：power(base, exp) @ src/calculator.py:10
  ✗ 签名不一致：div 文档(a, b, c) vs 代码(a, b) @ tests/test_calc.py:5
```

**问题清单即 AI 的修正任务**——约束驱动生成：AI 拿到清单修正 → 再 audit → 绿交付。

## 执行流程

```
1. 读契约（contract.yaml：代码/测试/文档路径 + 校验规则开关）
2. 提取代码 API 结构（AST）
3. 解析文档声明（docs 中的 API 清单）
4. 分析测试引用（静态分析）
5. 三边对比 → 问题清单
6. 退出码：0（对齐）/ 1（存在差异）
```

## 安全设计

- audit 只读：不修改代码/测试/文档
- 退出码可入 CI：合并前 audit 必须绿（约束生效）
- 问题清单机器可读（--json）——供 AI 直接消费

## 关联

- [index.md](index.md)：证据主线与架构总览
- 实验验证：`quanttide-laboratory-of-software-engineering/experiments/code-doc-code-loop`（代码↔文档边已实证）

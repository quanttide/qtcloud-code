# review → reflect → refactor

三层递进设计，以 review 为根基，向上叠加理解与修复能力。

## 人机协作模型

```
AI   = 海量初级程序员（听话、快速、不决策）
人类 = 高级程序员（定策略、审结果、做判断）
3R   = 高级对初级的分工接口
```

| 层面 | AI 角色 | 人类角色 |
|------|---------|----------|
| **review** | 全量扫描，报告所有可疑点 | 配置规则、排除误报 |
| **reflect** | 对每个 finding 补充上下文、影响分析、优先级 | 阅读分析结果，判断哪些需要处理 |
| **refactor** | 生成 patch（默认 dry-run） | 审核 patch，确认 apply |

人类通过选择启用哪些层（`--reflect`、`--refactor`、`--llm`）来制定策略，AI 在每层的约束内执行。

## 架构

```
                    LLM 可选 ─┐
                              │
review ──→ reflect ──→ refactor
   │           │           │
   │           │           └─ patch（差分输出）
   │           └─ 理解层（上下文 + 影响分析 + 解释）
   └─ 检测层（确定性规则引擎）
```

每层依赖下一层，不跨层调用。

---

## review

**已实现。** 确定性规则引擎。

输入：源码文件
输出：`Vec<Finding>`
可复现：同一文件同一版本 → 同一 finding 集合
LLM：永不接入

---

## reflect

**待设计。**

在 review 的 finding 上叠加理解层，不改 finding 集合。

```
reflect 输出 = finding +
  - 代码片段上下文（高亮问题行）
  - 影响范围（调用链、被引用处）
  - 重构优先级（结合调用频率、修改风险）
  - [--llm] 自然语言解释
```

### 纯语法部分（无需 LLM）

| 能力 | 实现方式 |
|------|----------|
| 代码上下文 | 从源码截取问题行附近 N 行 |
| 调用链 | tree-sitter 查询引用 |
| 影响范围 | 符号导出分析 |
| 优先级排序 | finding 级别 × 调用频率 |

### LLM 增强部分（`--llm`）

| 能力 | 输入 | 输出 |
|------|------|------|
| 问题解释 | finding + 代码片段 | "这个函数 80 行是因为前 40 行做验证，后 40 行做计算——建议提取 validate_input()" |
| 重构方向 | finding + 代码片段 | "重复代码片段可提取为 shared_helper()" |
| 误报说明 | finding + 代码片段 | "这里的 unsafe 是标准 FFI 模式，是安全的" |

### 实现约束

1. `reflect` 可以在无 `--llm` 时独立运行（纯语法分析）
2. `--llm` 只追加解释，不修改 finding
3. 纯语法部分放在 `reflect/` 模块，LLM 部分放在 `reflect/llm.rs`

---

## refactor

**需决策是否实现。**

输出可应用的 patch。与当前「做检测，不做自动修复」原则冲突，需修改原则后实现。

### 安全设计

```
refactor 输出 = patch（统一差异格式）

默认 --dry-run：输出 patch 不写文件
必须 --apply：才写入文件
每个 patch 对应一个 finding，可单独 apply/跳过
apply 后自动验证：cargo build + cargo test 通过才确认
```

### 可选路径

| 方案 | 做法 | 风险 |
|------|------|------|
| 不做 refactor | 保持当前原则 | 无 |
| 只做机械 refactor | 提取函数、重命名等纯语法操作 | 低 |
| 全量 refactor + LLM | LLM 生成重构代码 | 高：代码质量不可控 |

### 原则修改建议

如果决定实现 refactor：

```
- 做检测，不做自动修复
+ 默认不做自动修复，--refactor 可选启用
```

---

## 三层的 LLM 接入对比

| 层面 | LLM 角色 | 是否必需 |
|------|----------|----------|
| review | ❌ 不接入 | 否 |
| reflect | 解释已有 finding | 否（纯语法部分独立运行） |
| refactor | 生成重构代码 | 是（纯语法做不到有意义的重构） |

## 命令行设计

```
qtcloud-code review .                  # review 层
qtcloud-code review . --reflect        # review → reflect
qtcloud-code review . --reflect --llm  # review → reflect + LLM 解释
qtcloud-code review . --refactor       # review → reflect → refactor（需 --apply）
```

# refactor — 已移除

## 状态

**2026-08-16 已从交付约束体系移除。**

- 设计：修复层（提取函数/内联/死代码删除机械变换 + LLM 策略）
- 实现：仅 `rename`（符号重命名）——设计远大于实现
- 替代：修复主路径由 `review --mode deep`（LLM patch）与 AI 直接修正承担——refactor 命令被架空
- 处置：从体系移除；rename 能力如有需要并入独立工具

## 核心架构

```
finding + evidence_chain（reflect 输出）
  ↓
策略选择
  ├── 规则引擎（确定性）: "提取函数"、"重命名"、"内联变量"等 >80% 场景
  └── LLM（可选）: 需要理解代码意图时选策略并生成目标代码
  ↓
机械变换（规则引擎，确定性）
  ├── AST 模式匹配 → 函数提取
  ├── 符号操作 → 重命名、内联
  └── 依赖操作 → 移动符号、死代码删除
  ↓
patch → dry-run → [--apply] → 验证
```

**机械变换部分不需要 LLM**，是 IDE 每天都在做的事。LLM 只在策略选择和复杂代码生成时才需要。

## 机械变换引擎

| 变换 | 做法 | 确定性 | 覆盖场景 |
|------|------|--------|----------|
| **函数提取** | 选中代码块 → 抽为新函数 + 替换调用点 | ✅ | 过长函数 |
| **符号重命名** | 找到所有引用 → 统一替换 | ✅ | 命名不当 |
| **内联变量** | 展开变量引用 → 删除原声明 | ✅ | 冗余变量 |
| **内联函数** | 展开函数调用 → 删除原定义 | ✅ | 单次调用 |
| **死代码删除** | 检测未使用声明 → 移除 | ✅ | 未使用变量/函数 |
| **模块间移动** | 移动符号 → 更新所有导入路径 | ✅ | 模块拆分 |
| **重复代码合并** | 检测结构相似块 → 提取共享 | ⚠️ 边界需 LLM |

前六项纯规则引擎可做，是 IDE 如 IntelliJ、VS Code 的标配。

## 执行流程

```
reflect 输出（evidence_chain + 行动建议）
  ↓
规则引擎匹配变换策略
  ├── finding 类型 + 证据链 → 选择变换模板
  └── 例如 long-function → 函数提取
  ↓
机械变换（AST 操作）
  ├── 在 AST 中定位代码边界
  ├── 生成新函数
  └── 更新调用点
  ↓
patch 输出（默认 dry-run）
  ↓
[LLM 介入，可选]
  └── 当规则引擎无法确定策略时，LLM 选策略 + 生成代码
  ↓
人类审核 patch → 选择 apply 哪些
  ↓
自动验证 → 通过则确认，失败则回退
```

## 安全约束

```
1. dry-run 默认——只输出 patch 不写文件
2. --apply 显式确认——写入文件
3. 每个 finding 一个 patch——可单独 apply/跳过
4. apply 后自动验证——编译 + 测试不通过则回退
5. 验证失败标记——该 finding 标记为"需人工处理"，不再自动重试
```

## Patch 格式

```diff
--- a/src/main.rs
+++ b/src/main.rs
@@ -50,20 +50,25 @@
-fn run_review(...) {
+fn run_review(...) {
     let root = resolve_root(path)?;
+    scan_files(&root, &parsers, &detectors, &mut findings)
+}
+
+fn scan_files(root: &Path, parsers: &mut [...], detectors: &[...], findings: &mut Vec<Finding>) {
-    for entry in walkdir::WalkDir::new(&root)... {
-        scan_file(...);
-    }
+    for entry in walkdir::WalkDir::new(&root)... {
+        scan_file(...);
+    }
 }
```

Patch 元信息：

```json
{
  "finding_id": "long-function@src/main.rs:53",
  "rule_id": "long-function",
  "strategy": "extract-function",
  "engine": "rule",                        // rule | llm
  "risk": "low",
  "auto_verified": true,
  "files_changed": ["src/main.rs"]
}
```

## 风险分级

| 风险 | 引擎 | 条件 | 说明 |
|------|------|------|------|
| low | 规则 | 提取函数、重命名 | 纯机械操作，验证通过率高 |
| medium | 规则 | 拆分函数、移动代码 | 需理解代码逻辑 |
| high | LLM | 重写逻辑、更改 API | LLM 生成，需人工严格审核 |

## 命令行

```sh
review . --mode deep             # review + reflect + refactor（dry-run）
review . --mode deep --apply     # 审查 + 修复 + 写入
review . --apply "fc-*"          # 仅 apply 特定 finding（glob 匹配 finding_id）
```

## 回滚

```sh
review . --rollback "fc-*"       # 回退特定 patch
review . --rollback --last       # 回退上一次 apply 的所有 patch
```

每个 apply 会在 `.quanttide/code/refactor-log.jsonl` 记录，便于回滚。

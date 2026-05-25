# refactor — 修复层

## 职责

对 review + reflect 确定的 finding 生成修复代码。默认 dry-run，`--apply` 确认写入。

## 执行流程

```
reflect 输出（优先排序后的 finding）
  ↓
LLM 对每个 finding 生成 patch
  ├── 默认 dry-run（输出 diff，不改文件）
  ├── 人类在 dry-run 输出中选择 apply 哪些
  └── --apply 确认写入
  ↓
自动验证
  ├── 编译检查（cargo build / tsc / go build）
  └── 测试通过（cargo test / pytest / go test）
  ↓
验证通过 → 确认
验证失败 → 回退 patch，标记为"需人工处理"
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
+    // ... 原有的前置逻辑
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
  "risk": "low",
  "auto_verified": true,
  "files_changed": ["src/main.rs"]
}
```

## 风险分级

| 风险 | 条件 | 说明 |
|------|------|------|
| low | 提取函数、重命名 | 纯机械操作，验证通过率高 |
| medium | 拆分函数、移动代码 | 需要理解代码逻辑 |
| high | 重写逻辑、更改 API | LLM 可能生成 bug，需人工严格审核 |

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

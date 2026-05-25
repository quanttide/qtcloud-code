# AGENTS — qtcloud-code-cli

## 人机协作模型

3R = review → reflect → refactor。这是人类高级程序员的工作范式总结，不是工具要实现的系统架构。

```
人类（高级程序员）:
  review:   规则引擎做第一道扫描，辅助发现
  reflect:  根因追溯在我脑子里，工具提供证据
  refactor: 重构决策在我脑子里，工具负责执行

AI（初级程序员）:
  - 规则引擎 = lint 工具——快、确定、无遗漏
  - LLM = 辅助推理——理解语义，提供洞察
```

人类定策略：

```
--mode lint  仅规则引擎（秒级）
--mode llm   规则引擎 + LLM 审查（分钟级，默认）
--mode deep  规则引擎 + LLM + LLM 修复（需要审核）
```

## 发现分级

遵循 RFC 2119 语义：

| 级别 | 含义 | 举例 |
|------|------|------|
| **MUST** | 可能引入 bug，必须审查 | unsafe 块 >8 条 |
| **SHOULD** | 维护负担，建议重构 | 函数 >50 行 |
| **MAY** | 风格建议，可选采纳 | 函数 >30 行 |

同一规则可输出多个级别，取决于超标程度。例如函数 70 行输出 SHOULD，110 行输出 MUST。

## 架构

```
review --mode lint
  └─ 规则引擎扫描（快、确定）
  └─ 输出 finding

review --mode llm（默认）
  └─ 规则引擎扫描（同 lint）
  └─ LLM 二次审查
       ├── 优先级排序、去重
       ├── 上下文追加
       └── 纯 LLM 规则（安全漏洞、并发 bug 等语义问题）

review --mode deep
  └─ 规则引擎 + LLM 审查（同 llm）
  └─ LLM 生成修复 patch
       └── dry-run 默认，--apply 确认
```

## 规则引擎定位

规则引擎不是主力，是安全网：

| 场景 | 职责 |
|------|------|
| LLM 遗漏了 | 规则引擎兜底，不放过任何已知模式 |
| LLM 误判了 | 规则引擎给出确定性证据 |
| 无 LLM 时 | 规则引擎独立运行，模式退化为 lint |
| 确定性基线 | 无论 LLM 版本如何，lint 结果一致 |

## 检测器分类

| 类型 | 执行引擎 | 举例 |
|------|----------|------|
| **语法规则** | 规则引擎（tree-sitter） | 过长函数、unsafe 块、过长参数列表 |
| **编译规则** | 规则引擎（cargo check） | 未使用变量 |
| **项目规则** | 规则引擎（文件映射） | 缺失测试 |
| **语义规则** | LLM 审查 | 安全漏洞、并发 bug、逻辑错误 |

### 跨语言检测注意事项

不同语言 tree-sitter 节点结构差异大，检测器需处理：
- **Rust** `function_item` → `parameters` → `parameter`（每个参数独立节点）
- **Python** `function_definition` → `parameters`（与 Rust 结构兼容）
- **Go** `function_declaration` → `parameters` → `parameter_declaration` → 多个 `identifier`（共享类型声明）
- **Dart** `function_declaration` → `function_signature` → `identifier`（函数名在孙子节点）
- **TypeScript** 同 Go/Dart 的 `function_declaration` 结构

优先使用 `child_by_field_name("parameters")`，必须为各语言准备 fallback。

### 配置驱动排除

三层过滤减少检测噪音：
1. 硬编码跳过（`target/`、`.git/`、非源码扩展名）
2. 启发式判断（inline test、external test file）
3. 用户配置排除（`.quanttide/code/contract.yaml` 的 `exclude` 字段）

## 测试

```sh
# 单元测试 + 集成测试
cargo test

# 覆盖率（目标 >90%）
cargo llvm-cov
```

### 覆盖策略

基准：**总体行覆盖 ≥ 90%**（当前 92%）

| 类型 | 目标 | 方法 |
|------|------|------|
| **纯函数** | ~100% | 直接测阈值、解析逻辑 |
| **文件级检测器** | >90% | 各语言 parser + 场景覆盖 |
| **项目级检测器** | >90% | 拆出纯函数单独测 |
| **CLI 错误路径** | ~80% | 集成测试覆盖主要路径，余留 5% 不追 |

## 模块结构

```
src/
├── main.rs          # CLI 入口 (clap)
├── lib.rs           # 公开模块
├── config.rs        # .quanttide/code/contract.yaml 配置加载
├── parser/          # 语言解析器
│   ├── mod.rs       # LanguageParser trait + ParseResult
│   ├── rust.rs      # RustParser
│   ├── python.rs    # PythonParser
│   ├── go.rs        # GoParser
│   ├── dart.rs      # DartParser
│   └── typescript.rs # TypeScriptParser + TsxParser
├── detector/        # 检测器
│   ├── mod.rs       # Detector trait + Finding + walk_tree
│   ├── long_function.rs
│   ├── long_parameter_list.rs
│   ├── unsafe_block.rs
│   ├── unused_variable.rs
│   └── missing_tests.rs
├── output.rs        # 输出格式：JSON / Terminal / STATUS.md
```

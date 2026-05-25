# AGENTS — qtcloud-code-cli

## 架构原则

- 纯规则引擎，不依赖 LLM
- 面向「可检测、可复现、可自动化」
- P0 阶段只做 Rust，再扩展多语言（已支持 Python/Go/Dart/TypeScript）
- 做检测，不做自动修复
- 自举（`qtcloud-code review .`）驱动质量反馈

## 发现分级

遵循 RFC 2119 语义：

| 级别 | 含义 | 举例 |
|------|------|------|
| **MUST** | 可能引入 bug，必须审查 | unsafe 块 >8 条 |
| **SHOULD** | 维护负担，建议重构 | 函数 >50 行 |
| **MAY** | 风格建议，可选采纳 | 函数 >30 行 |

同一规则可输出多个级别，取决于超标程度。例如函数 70 行输出 SHOULD，110 行输出 MUST。

## 检测器分类

| 类型 | 特征 | 举例 |
|------|------|------|
| **文件级** | 遍历单文件 AST，统一用 `walk_tree` | 过长函数、unsafe 块、过长参数列表 |
| **项目级** | 需要项目上下文，独立于文件循环调用 | 未使用变量（`cargo check`）、缺失测试 |

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
├── lang/            # 语言解析器抽象与实现
│   ├── mod.rs       # LanguageParser trait + ParseResult
│   ├── rust.rs      # RustParser
│   ├── python.rs    # PythonParser
│   ├── go.rs        # GoParser
│   ├── dart.rs      # DartParser
│   └── typescript.rs # TypeScriptParser + TsxParser
├── detect/          # 检测器
│   ├── mod.rs       # Detector trait + Finding + walk_tree
│   ├── long_function.rs
│   ├── long_parameter_list.rs
│   ├── unsafe_block.rs
│   ├── unused_variable.rs   # 项目级：cargo check 解析
│   └── missing_tests.rs      # 项目级：源文件/测试映射
└── report/          # 输出格式
    └── mod.rs       # JSON / Terminal / STATUS.md
```

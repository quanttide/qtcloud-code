# qtcloud-code-cli

多语言代码静态分析与质量检测 CLI。

纯规则引擎 + tree-sitter AST 分析 + cargo check 集成。**review（已发布）→ reflect（开发中）→ refactor（预览版）**。

## 安装

```sh
# 从 crates.io 安装（推荐）
cargo install qtcloud-code-cli

# 或从源码编译
cd apps/qtcloud-code/src/cli
cargo install --path .
```

或直接运行：

```sh
cargo run -- review <path>
```

## 使用

```sh
# 审查目标目录
qtcloud-code review src/

# JSON 格式输出
qtcloud-code review . --format json

# 仅运行指定规则
qtcloud-code review . --rules long-function,long-parameter-list

# 写入 STATUS.md
qtcloud-code review . --status
```

### 配置管理

```sh
# 创建默认配置文件
qtcloud-code contract init

# 列出可用规则
qtcloud-code contract list
qtcloud-code contract list --json

# 校验配置
qtcloud-code contract validate
```

### 代码变换（预览版）

```sh
# 重命名符号（仅预览，不写入文件）
qtcloud-code refactor rename <file> --old-name foo --new-name bar
```

## 检测规则

| 规则 | 级别 | 说明 |
|------|------|------|
| `long-function` | MAY/SHOULD/MUST | 函数体超过 30/50/80 行 |
| `long-parameter-list` | MAY/SHOULD/MUST | 参数超过 4/6/9 个 |
| `rust-wide-unsafe` | MAY/SHOULD/MUST | unsafe 块超过 3/5/8 条 |
| `unused-variable` | SHOULD | 未使用变量（cargo check） |
| `missing-tests` | MUST | 源文件缺少对应测试 |

> 当前检测器主要覆盖 Rust 语法节点。Python/Go/Dart/TypeScript 文件能被正确解析，但检测结果由语言通用规则（过长函数、过长参数列表）产出，Rust 专用规则（unsafe 块、cargo check）不适用于其他语言。

## 配置

`.quanttide/code/contract.yaml`：

```yaml
code:
  rules:
    - long-function
    - long-parameter-list
    - missing-tests
  exclude:
    - src/main.rs
    - src/**/mod.rs
```

`--rules` CLI 参数优先级高于配置文件。

## 支持的语言

| 语言 | 解析器 | 扩展名 | 检测器覆盖 |
|------|--------|--------|-----------|
| Rust | `tree-sitter-rust` | `.rs` | 全部规则 |
| Python | `tree-sitter-python` | `.py` | 通用规则 |
| Go | `tree-sitter-go` | `.go` | 通用规则 |
| Dart | `tree-sitter-dart` | `.dart` | 通用规则 |
| TypeScript | `tree-sitter-typescript` | `.ts`, `.tsx` | 通用规则 |

## 开发

```sh
# 测试 + 覆盖率
cargo test
cargo llvm-cov

# 自举验证
cargo run -- review .

# 覆盖率基准（线覆盖 ≥ 90%）
cargo llvm-cov test --lcov --output-path lcov.info
```

## 许可

Apache 2.0

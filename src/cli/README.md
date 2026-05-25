# qtcloud-code-cli

多语言代码静态分析与质量检测 CLI。

**qtcloud-code** 是一个纯规则引擎的代码审查工具，支持 5 种语言（Rust / Python / Go / Dart / TypeScript）的语法级和编译器级检测。不依赖 LLM，面向可检测、可复现、可自动化。

## 安装

```sh
cargo install --path .
```

或直接用 Cargo 运行：

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

## 检测规则

### 语法级（AST 分析，多语言通用）

| 规则 | 级别 | 说明 |
|------|------|------|
| `long-function` | MAY / SHOULD / MUST | 函数体超过 30/50/80 行 |
| `long-parameter-list` | MAY / SHOULD / MUST | 参数超过 4/6/9 个 |
| `rust-wide-unsafe` | MAY / SHOULD / MUST | Rust unsafe 块超过 3/5/8 条语句 |

### 编译器级（项目级检测）

| 规则 | 级别 | 说明 |
|------|------|------|
| `unused-variable` | SHOULD | 未使用变量（通过 `cargo check` 解析） |
| `missing-tests` | MUST | 源文件缺少对应测试 |

## 配置

在项目根目录创建 `.quanttide/code/contract.yaml`：

```yaml
code:
  # 选择启用的规则
  rules:
    - long-function
    - long-parameter-list
    - missing-tests
  # 排除无需测试的文件
  exclude:
    - src/main.rs
    - src/**/mod.rs
```

`--rules` CLI 参数优先级高于配置文件。

## 支持的语言

| 语言 | 解析器 | 扩展名 |
|------|--------|--------|
| Rust | `tree-sitter-rust` | `.rs` |
| Python | `tree-sitter-python` | `.py` |
| Go | `tree-sitter-go` | `.go` |
| Dart | `tree-sitter-dart` | `.dart` |
| TypeScript | `tree-sitter-typescript` | `.ts` |
| TSX | 同上 | `.tsx` |

## 开发

```sh
# 测试
cargo test

# 自举验证（用自己扫描自己）
cargo run -- review .

# 覆盖率
cargo llvm-cov
```

## 许可

Apache 2.0

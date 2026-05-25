# qtcloud-code-cli

多语言代码静态分析与质量检测 CLI。

**qtcloud-code** 是一个 3R 代码审查 CLI：**review → reflect → refactor**。规则引擎是安全网，LLM 是干活的主力（开发中）。支持 5 种语言，面向可检测、可复现、可自动化。

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

### 代码变换

```sh
# 应用 patch（默认写入，--dry-run 预览）
qtcloud-code refactor apply <file> --line <N>
qtcloud-code refactor apply <file> --line <N> --dry-run

# 重命名符号
qtcloud-code refactor rename <file> --old-name foo --new-name bar
qtcloud-code refactor rename <file> --old-name foo --new-name bar --dry-run
```

## 检测规则

| 规则 | 级别 | 引擎 | 说明 |
|------|------|------|------|
| `long-function` | MAY/SHOULD/MUST | tree-sitter | 函数体超过 30/50/80 行 |
| `long-parameter-list` | MAY/SHOULD/MUST | tree-sitter | 参数超过 4/6/9 个 |
| `rust-wide-unsafe` | MAY/SHOULD/MUST | tree-sitter | Rust unsafe 块超过 3/5/8 条 |
| `unused-variable` | SHOULD | cargo check | 未使用变量 |
| `missing-tests` | MUST | 文件映射 | 源文件缺少对应测试 |

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

| 语言 | 解析器 | 扩展名 |
|------|--------|--------|
| Rust | `tree-sitter-rust` | `.rs` |
| Python | `tree-sitter-python` | `.py` |
| Go | `tree-sitter-go` | `.go` |
| Dart | `tree-sitter-dart` | `.dart` |
| TypeScript | `tree-sitter-typescript` | `.ts`, `.tsx` |

## 开发

```sh
# 测试 + 覆盖率
cargo test
cargo llvm-cov

# 自举验证
cargo run -- review .

# 覆盖率基准
cargo llvm-cov test --lcov --output-path lcov.info
# 线覆盖 ≥ 90%
```

## 架构

```
review（规则引擎）
├── 语法规则：过长函数、unsafe 块、过长参数列表
├── 编译规则：未使用变量（cargo check）
├── 项目规则：缺失测试
└── 依赖图规则：循环依赖/高扇入扇出/孤立模块

reflect（侦探追溯）— 开发中
├── 程序切片：反向追溯影响语句
└── 数据流分析：变量赋值链

refactor（代码变换）
├── apply：dry-run + 写入 + 自动验证
├── rename：符号表 + 重命名
└── 安全机制：Patch、操作日志
```

## 许可

Apache 2.0

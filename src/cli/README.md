# qtcloud-code-cli

多语言代码静态分析与质量检测 CLI。

**约束驱动生成**：约束 AI 编码交付的代码、测试、文档三者对齐且质量达标。

```
约束（契约定义）──→ AI 生成（代码+测试+文档）──→ audit + review 校验
     ↑                                              │
     └──────────── 问题清单反馈 → AI 直接修正 ────────┘
```

- **audit**（对齐约束）：代码 ↔ 测试 ↔ 文档三角对齐，机器可判定
- **review**（质量约束）：规则引擎 + LLM 二次审查
- **reflect**（独立工具）：定向代码分析（slice / trace / graph / suggest）

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

# 审查模式：lint（仅规则引擎）/ llm（规则引擎+LLM，默认）/ deep
qtcloud-code review . --mode lint

# 写入 STATUS.md
qtcloud-code review . --status
```

### 对齐审计（约束驱动生成的核心）

```sh
# 校验代码、测试、文档三者对齐（退出码 0 对齐 / 1 存在差异，可入 CI）
qtcloud-code audit .

# JSON 输出（机器可读，供 AI 直接消费）
qtcloud-code audit . --json
```

audit 校验三条边：

| 边 | 校验内容 | 机制 |
|----|---------|------|
| 代码 ↔ 文档 | API 结构一致（函数/签名/参数） | 提取器（AST）↔ 文档声明对比 |
| 代码 ↔ 测试 | 测试引用的 API 存在且签名一致 | 静态分析测试引用 ↔ 代码导出 |
| 测试 ↔ 文档 | 文档声明的行为有测试覆盖 | 文档声明集合 ↔ 测试引用集合 |

输出结构化问题清单 `{类型, API, 位置, 期望, 实际}`——清单即 AI 的修正任务。

### 骨架生成（文档驱动 / 测试驱动）

audit 校验闭环的另一半是生成：从文档或测试生成骨架，让 AI/人按问题清单填充。

```sh
# 文档驱动：文档声明 → 测试骨架（语言自动从文档代码块检测，或用 --lang 指定）
qtcloud-code scaffold tests docs/api.md
qtcloud-code scaffold tests docs/api.md --lang py --output tests/test_calc.py

# 测试驱动：测试引用 → 代码骨架（stub，语言从扩展名推断）
qtcloud-code scaffold code tests/test_calc.py
qtcloud-code scaffold code tests/test_calc.rs --output src/calc.rs
```

**文档驱动流程**：写文档声明 API → `scaffold tests` 生成测试骨架 → 实现代码 → `audit` 绿交付

**测试驱动流程（TDD）**：先写测试 → `scaffold code` 生成代码骨架 → 填充实现 → `audit` 绿交付

audit 红态输出会提示下一步：测试引用未实现 API 时提示 `scaffold code`，文档声明缺测试时提示 `scaffold tests`。

### LLM 审查配置（review --mode llm/deep）

通过环境变量配置 OpenAI 兼容接口，未配置时自动回退 lint 模式：

```sh
export QTTCODE_LLM_API_KEY=sk-xxx
export QTTCODE_LLM_BASE_URL=https://api.openai.com/v1   # 可选
export QTTCODE_LLM_MODEL=gpt-4o-mini                     # 可选
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
# 重命名符号（默认写入，--dry-run 预览）
qtcloud-code refactor rename <file> --old-name foo --new-name bar
qtcloud-code refactor rename <file> --old-name foo --new-name bar --dry-run
```

### 定向分析（reflect）

```sh
# 反向追溯：某行结果依赖了哪些变量
qtcloud-code reflect slice <file> <line>

# 变量数据流：声明 → 使用
qtcloud-code reflect trace <file> <var> [line]

# 函数级调用图
qtcloud-code reflect graph <file>

# 推荐可疑行（return / panic / unsafe / cast / parse）
qtcloud-code reflect suggest <file>

# 所有子命令支持 --json 输出
qtcloud-code reflect slice <file> <line> --json
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
audit:
  code: [src]        # 代码目录/文件
  tests: [tests]     # 测试目录/文件
  docs: [docs]       # 文档目录/文件
  edges:             # 启用的校验边（默认全部）
    - code-docs
    - code-tests
    - tests-docs
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
cargo run -- audit .

# 覆盖率基准（线覆盖 ≥ 90%）
cargo llvm-cov test --lcov --output-path lcov.info
```

## 许可

Apache 2.0

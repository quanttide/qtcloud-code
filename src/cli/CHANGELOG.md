# Changelog

## [0.3.1] — 2026-08-16

- **`scaffold` 命令：文档驱动 / 测试驱动骨架生成**（与 audit 组成约束驱动生成闭环）
  - `scaffold tests <文档>`：从文档声明的 API 生成测试骨架（语言自动从代码块检测或 `--lang`）
  - `scaffold code <测试>`：从测试引用生成代码骨架（stub，外部/内置调用自动过滤）
  - 支持 Rust / Python / Go / TypeScript；`--output` 写入文件
- audit 红态输出驱动提示：测试引用未实现 → 提示 `scaffold code`；文档声明缺测试 → 提示 `scaffold tests`
- `audit::project_refs` 公开：外部/内置调用过滤供 scaffold 与边 2 共用
- 集成测试：`tests/scaffold.rs` 13 个（含文档驱动 → 测试骨架 → 代码骨架 → audit 绿 的完整闭环）

## [0.3.0] — 2026-08-16

- **`audit` 命令（核心）**：对齐审计——校验代码、测试、文档三角对齐（约束驱动生成）
  - 边 1 代码 ↔ 文档：API 结构一致（AST 提取 ↔ 文档声明对比）
  - 边 2 代码 ↔ 测试：测试引用的 API 存在且签名一致
  - 边 3 测试 ↔ 文档：文档声明的行为有测试覆盖
  - 输出结构化问题清单 `{类型, API, 位置, 期望, 实际}`——即 AI 的修正任务
  - 退出码 0（对齐）/ 1（差异）可入 CI；`--json` 机器可读；只读安全
  - 契约驱动：contract.yaml 的 `audit` 段（code/tests/docs 路径 + edges 开关）
- `review --mode lint|llm|deep`：LLM 二次审查层（优先级/解释/置信度 + 语义 finding），
  环境变量 `QTTCODE_LLM_API_KEY` 等配置 OpenAI 兼容接口，未配置自动回退 lint
- review JSON 输出对齐 docs/dev/review.md 格式：`{mode, engine, llm, findings}`
- `reflect` 子命令全部支持 `--json` 输出
- 集成测试补齐：audit（绿/红项目、契约路径、exclude、多语言）、reflect（json/错误路径/Go）、review（mode）
- 新增依赖：`ureq`（LLM 审查 HTTP 客户端）

## [0.2.1] — 2026-05-26

- `reflect` 子命令：slice / trace / graph / suggest，多语言支持（Rust / Python / Go / TypeScript）
- 12 个 reflect 集成测试
- P1.5 完善：移除 `_json` 死参数，消除编译 warning，修复 unsafe 检测条件
- 新增 STATUS.md 和 TODO.md 项目文档

## [0.2.0] — 2026-05-26

- `contract` 命令：init / list / validate / list --json
- `refactor rename`：符号表 + 实际文件写入 + `--dry-run`
- 规则引擎加强：覆盖基准 90%（当前 93%）
- 发布流水线：release → build-cli → release-cli → crates.io
- 89 测试，移除实验代码

## [0.2.0-rc.3] — 2026-05-26

- 验证 release-cli（`cargo publish`）工作流
- Cargo.toml version 对齐 `0.2.0-rc.3`

## [0.2.0-rc.2] — 2026-05-26

- `contract` 命令：init / list / validate
- `refactor rename`：符号表 + 实际文件写入 + `--dry-run`
- `list-rules --json` 输出
- 89 测试，覆盖率 93%
- 移除实验代码（dead_code、revert、apply）

## [0.2.0-rc.1] — 2026-05-26

- CI 验证（CHANGELOG 未提交导致 check 失败）

## [0.1.0] — 2026-05-25

首个正式版本。

- 5 语言解析：Rust / Python / Go / Dart / TypeScript (TSX)
- 5 检测器：过长函数、unsafe 块、过长参数列表、未使用变量、缺失测试
- 配置：`.quanttide/code/contract.yaml` + `--rules` 过滤
- 发布到 crates.io (v0.1.0)

## [0.1.0-rc.6] — 2026-05-25

修复 license 格式 `Apache 2.0` → `Apache-2.0`。

## [0.1.0-rc.5] — 2026-05-25

验证 build→release / release→build 触发链。

## [0.1.0-rc.4] — 2026-05-25

验证 release-cli 改用 release 事件触发。

## [0.1.0-rc.3] — 2026-05-25

验证 release-cli 仅在 tag push 触发。

## [0.1.0-rc.2] — 2026-05-25

实验 CI 流水线。工作流对齐 `cli/v*` tag 格式 + 三平台二进制构建。

## [0.1.0-rc.1] — 2026-05-25

首个预发布版本。目标：

- 验证 Rust 项目结构与 CI 发布管道
- 验证跨语言检测器架构在真实代码上的表现
- 收集实际使用反馈

功能概览：

- 5 语言解析：Rust / Python / Go / Dart / TypeScript (TSX)
- 5 检测器：过长函数、unsafe 块、过长参数列表、未使用变量、缺失测试
- 配置：`.quanttide/code/contract.yaml` + `--rules` 过滤
- 自举验证零 MUST/SHOULD，77 测试，覆盖率 95%

# Changelog

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

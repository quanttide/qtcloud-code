# Changelog

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

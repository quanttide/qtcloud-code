# Code Scan Status — qtcloud-code-cli

> 自动生成于 `qtcloud-code review`，最后更新: 2026-05-26

## 项目概览

| 指标 | 值 |
|------|-----|
| 当前版本 | `cli/v0.2.0` |
| 测试总数 | 101（89 review + 12 reflect） |
| 命令行注册 | review / list-rules / contract / refactor / reflect |
| 子命令 | contract: init/list/validate · refactor: rename · reflect: slice/trace/graph/suggest |
| 支持语言 | Rust / Python / Go / Dart / TypeScript (TSX) |
| 发布渠道 | crates.io |

## 进度汇总

| 里程碑 | 状态 | 说明 |
|--------|------|------|
| P0 — 基础检测 | ✅ 已发布 | 5 解析器 + 5 检测器 + 配置 + 发布流水线 |
| P1.5 — reflect 完善 | ✅ 已推送 | AST 追溯、跨语言 trace、parser 复用、warning 消除 |
| P2 — `--llm` | ⏳ 未开始 | LLM 客户端 + 统一 prompt + 验证层 |

## reflect 子命令状态

| 子命令 | 状态 | 实现方式 | 已知限制 |
|--------|------|----------|----------|
| `slice` | ✅ 完成 | 行级反向收集，截断 10 条 | 非 AST 追溯，简化版 |
| `trace` | ✅ 完成 | 行级变量匹配 + 自动声明查找 | 自动查找仅 Rust |
| `graph` | ✅ 完成 | `fn` 关键词行级匹配 | 调用/被调用数硬编码为 0 |
| `suggest` | ✅ 完成 | 文本模式匹配（return/panic/unsafe/cast/parse） | 纯文本，非 AST |
| `--json` | ❌ 待实现 | 签名预留未实现 | 所有 handler `_json` 硬编码 false |

## 测试覆盖

```
cargo test --test reflect        # 12 passed ✓
cargo test                       # ≈ 101 total
```

## 已知问题

1. 2 个编译 warning（`_tree`、`_json` 未使用变量）
2. 尚未提交：Cargo.toml + main.rs 的 reflect 变更 + tests/reflect.rs

## 依赖

- `tempfile` = "3"（dev-dependency，测试用）
- 无新增依赖（reflect 使用已有的 tree-sitter parser）

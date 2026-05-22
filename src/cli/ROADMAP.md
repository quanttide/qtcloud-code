# ROADMAP

## v0.1.0 — `audit` 命令上线

- [x] `qtcloud-code audit <source>` — 对指定目录执行代码审计（ruff + lizard）
- [ ] 生成 JSON 格式审计报告
- [ ] 支持 `--fix` 自动修复 ruff 问题

## 后续规划

- `qtcloud-code audit --ci` — CI 模式（只输出警告行，不输出完整报告）
- `qtcloud-code lint` — 仅运行 ruff
- `qtcloud-code complexity` — 仅运行 lizard
- 支持更多编程语言（Go、TypeScript）

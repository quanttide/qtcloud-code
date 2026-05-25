# TODO — qtcloud-code-cli

## P0 已完成 ✅

- [x] `review` 命令（5 语言 + 5 检测器）
- [x] `contract` 命令（init / list / validate）
- [x] `refactor rename`（符号表 + 写入 + `--dry-run`）
- [x] 配置系统 + 输出格式 + 发布流水线
- [x] 89 测试，覆盖率 93%

## P1 — Reflect 工具集成

### 从实验室集成 reflect 工具
- [ ] 集成 `backward_slice`（`reflect/slice.rs`→ `src/reflect/slice.rs`）
- [ ] 集成 `forward_slice`
- [ ] 集成 `flatten_stmts`
- [ ] 集成 `call_graph`（`reflect/analysis.rs`）
- [ ] 集成 `type_info`

### `--reflect` 标志
- [ ] `review --reflect` 标志
- [ ] `analyse` 子命令（`analyse <file> --line <N>`）
- [ ] 工具输出格式化（JSON + 终端）

## P2 — LLM 集成

### 基础设施
- [ ] LLM 客户端 trait（`send_prompt`）
- [ ] DeepSeek 实现
- [ ] Vault 密钥读取
- [ ] `--llm` CLI 标志

### Reflect + LLM
- [ ] finding + evidence → LLM 根因分析 prompt
- [ ] prompt 模板：安全分析（backward_slice + dataflow）
- [ ] prompt 模板：重复识别（flatten_stmts）
- [ ] prompt 模板：一致性检查（dataflow × N）

### Refactor + LLM
- [ ] LLM 生成 target code（提取函数/拆分）
- [ ] `--mode deep` 标志（review → reflect → refactor）
- [ ] 安全机制（dry-run / apply / 验证）
- [ ] 验证闭环（refactor 后自动 re-review）

## P3 — CodeAgent 循环
- [ ] 多轮修复（直到零 MUST）
- [ ] 修复记忆
- [ ] 交互式审核
- [ ] pre-commit hook

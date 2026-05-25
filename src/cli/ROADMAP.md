# ROADMAP — qtcloud-code-cli

## 定位

基于 Reflexion 的 3R CodeAgent：**Review → Reflect → Refactor**，构成闭环。

```
Review（检测）→ finding → Reflect（理解）→ 根因 → Refactor（修复）→ Review（验证）
```

纯规则引擎 + reflect 工具（确定性）+ LLM（非确定推理，可选）。LLM 不参与时退化为纯 review。

## P0 — 已完成 ✅

- [x] CLI 框架（`review` / `contract` / `refactor rename`）
- [x] 5 语言解析（Rust / Python / Go / Dart / TypeScript）
- [x] 5 检测器（过长函数 / unsafe 块 / 过长参数列表 / 未使用变量 / 缺失测试）
- [x] 配置系统（`.quanttide/code/contract.yaml` + `--rules`）
- [x] 输出格式（终端 / JSON / STATUS.md）
- [x] `contract` 命令（init / list / validate）
- [x] `refactor rename`（符号表 + 实际文件写入 + `--dry-run`）
- [x] 发布流水线（`cli/v*` → crates.io）
- [x] 89 测试，覆盖率 93%

## P1 — Reflect 工具集成

reflect 层的确定性工具（实验室已验证）集成到正式 CLI，构成 CodeAgent 的「Observe」阶段。

### 从实验室集成

- [ ] `backward_slice`：从 finding 行号反向追溯变量定义链
- [ ] `forward_slice`：从定义点找出所有使用位置
- [ ] `flatten_stmts`：展平函数体语句
- [ ] `call_graph`：函数级调用关系图
- [ ] `type_info`：变量类型注解提取

### `--reflect` 标志

```sh
qtcloud-code review . --reflect     # review + reflect 工具分析
qtcloud-code analyse f.rs --line 10 # 定向分析（不运行 review）
```

- [ ] `--reflect` CLI 标志
- [ ] `analyse` 子命令（定向分析）
- [ ] 工具输出格式化（证据链）

## P2 — LLM 集成（Reflect 推理 + Refactor 修复）

LLM 接入 Reflexion 循环，提供非确定推理和代码生成能力。

### 基础设施

- [ ] LLM 客户端（从实验室 llm.rs 集成，支持 DeepSeek）
- [ ] Vault 密钥管理
- [ ] `--llm` CLI 标志

### Reflect + LLM

- [ ] finding + evidence → LLM 根因分析
- [ ] 跨 finding 根因分析（项目级）
- [ ] prompt 模板注册（安全分析 / 重复识别 / 一致性检查）

### Refactor + LLM

- [ ] LLM 生成 target code
- [ ] `--mode deep` 标志（review → reflect → refactor）
- [ ] 安全机制集成（dry-run / apply / 验证）
- [ ] 验证闭环（refactor 后自动 review）

## P3 — CodeAgent 完整循环

- [ ] 多轮修复：review → reflect → refactor → review（直到零 MUST）
- [ ] 修复记忆：跨轮次上下文保持
- [ ] 交互式审核：逐条确认 finding 和 patch
- [ ] pre-commit hook 模式

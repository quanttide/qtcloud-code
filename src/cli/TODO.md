# TODO — qtcloud-code-cli

## P0 已完成 ✅

- [x] CLI 框架（`review` / `list-rules`）
- [x] 多语言解析（Rust / Python / Go / Dart / TypeScript）
- [x] 5 检测器
- [x] 配置系统 + `--rules`
- [x] 输出格式（终端 / JSON / STATUS.md）
- [x] 119 测试
- [x] 发布流水线（`cli/v*` → crates.io）

## P1 — `--llm` 模式

### LLM 客户端
- [ ] 定义 LLM 客户端 trait（`send_prompt`、`stream_response`）
- [ ] 实现 OpenAI/Claude API 适配器
- [ ] `--llm` CLI 标志 + API key 配置

### finding 增强
- [ ] 定义 LLM 审查 prompt（输入：代码 + finding，输出：优先级/解释/确认）
- [ ] 实现 LLM 二次审查流程（按文件批处理）
- [ ] 增强 finding 结构（追加 `llm_priority`、`llm_explanation`、`confidence`）

### 纯 LLM 规则
- [ ] 纯 LLM 规则框架（输入：文件 + 无 finding，输出：语义问题）
- [ ] 安全漏洞规则 prompt
- [ ] 并发 bug 规则 prompt

### 成本控制
- [ ] 只对有 finding 的文件发起 LLM 调用
- [ ] 结果缓存（同一文件同一版本 24h）
- [ ] 失败重试 + 降级（LLM 失败回退到规则引擎结果）

## P2 — `--mode deep`

### 安全机制（从实验室 safety.rs 集成）
- [ ] 集成 `Patch` 结构
- [ ] 集成 `dry_run()` 生成 diff
- [ ] 集成 `apply_patch()` 写文件 + 验证
- [ ] 集成 `rollback()` 回滚
- [ ] 集成 `OperationLog` 操作日志
- [ ] `--mode deep` CLI 标志
- [ ] `--apply` 确认写入标志
- [ ] 自动验证（编译 + 测试）

### 符号表（从实验室 rename.rs 集成）
- [ ] 集成 `SymbolTable` 结构
- [ ] 集成 `build_symbol_table()`（扫描项目）
- [ ] 集成 `rename_symbol()`（生成替换映射）
- [ ] 扩展为跨文件符号表

## P3 — `contract` 命令
- [ ] `contract init` 交互式创建配置
- [ ] `contract list` 替换 `list-rules`（JSON 输出）
- [ ] `contract validate` 校验配置 vs 规则
- [ ] 规则元数据抽取为独立模块

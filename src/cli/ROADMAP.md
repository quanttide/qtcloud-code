# ROADMAP — qtcloud-code-cli

规则引擎是辅助，不是替代。做检测，不做框架。

## P0 — 已发布 ✅

- [x] 5 语言解析 + 5 检测器
- [x] `contract` 命令（init / list / validate）
- [x] `refactor rename`（符号表 + 写入 + `--dry-run`）
- [x] 配置系统 + 输出格式 + 发布流水线
- [x] 89 测试，覆盖率 93%
- [x] crates.io: v0.1.0, v0.2.0

## P1 — 实验室成熟工具整合 ✅

- [x] `reflect` 子命令：slice / trace / graph / suggest
- [x] 多语言支持（Rust / Python / Go / TypeScript）
- [x] 返回值统一三态退出码（0=有结果 / 1=无结果 / 2=错误）
- [x] 12 个集成测试全部通过
- [x] 代码评审 + 修复（unsafe 检测条件、临时文件清理）
- [x] 设计文档 `docs/reflect.md` + 测试设计 `docs/api/reflect-integration-test.md`

**实验室已验证（已纳入设计）：**
- 精确截断点：最短证据（1 行输出端）发现最多问题 ✅
- 反面案例：全函数策略分析 + 片段边界聚焦的**双通道策略** ✅
- 统一 prompt：dismiss 不降级 + 免费额外发现，可作 `--llm` 默认 ✅

### 已知限制（后续迭代）

- `--json` 未实现，handlers 签名有 `_json` 参数但硬编码 false
- `slice` 是行级简化版，非真正的 tree-sitter AST 追溯
- `trace` 自动查找声明仅支持 Rust（`let var` 模式匹配）
- `graph` 只做 `fn` 关键词行级匹配，不生成真实调用图
- 2 个编译 warning（`_tree`、`_json` 未使用）

## P2 — `--llm`

```
qtcloud-code review . --llm    # 规则引擎 + LLM 分析
```

- [ ] LLM 客户端 + `--llm` 标志
- [ ] 统一 prompt（dismiss + discover）作为默认 prompt
- [ ] 规则引擎验证层：LLM 诊断 → 匹配 VerificationRule（实验室 83% 可验证）
- [ ] 双通道策略：先全函数策略分析，再用可疑行号做片段聚焦
- [ ] 置信度标注（证据锚定率 + 规则引擎交叉验证）

## 非目标

- 不做框架、不做编排、不做「智能体」
- 不做跨 finding 根因分析（那是人的工作，工具提供证据就够了）

# ROADMAP — qtcloud-code-cli

规则引擎是辅助，不是替代。做检测，不做框架。

## P0 — 已发布 ✅

- [x] 5 语言解析 + 5 检测器
- [x] `contract` 命令（init / list / validate）
- [x] `refactor rename`（符号表 + 写入 + `--dry-run`）
- [x] 配置系统 + 输出格式 + 发布流水线
- [x] 89 测试，覆盖率 93%
- [x] crates.io: v0.1.0, v0.2.0

## P1 — 实验室成熟工具整合

```
qtcloud-code reflect <file> --line 10   # 定向分析
```

- [ ] `backward_slice` + `flatten_stmts`：从实验室 examples/default/src/reflect/ 移植到 src/cli
- [ ] `reflect` 子命令：slice/trace/graph/suggest/scan，多语言支持（实验室原型已有 ➡️ 移植 + 适配）
- [ ] clap 子命令注册 + 测试

**实验室已验证：**
- 精确截断点：最短证据（1 行输出端）发现最多问题 ✅
- 反面案例：全函数策略分析 + 片段边界聚焦的**双通道策略** ✅
- 统一 prompt：dismiss 不降级 + 免费额外发现，可作 `--llm` 默认 ✅

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

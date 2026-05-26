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

从 lab 验证过的工具中，挑确定性强、贴合真实场景的整合进来。

```
qtcloud-code analyse <file> --line 10   # 定向分析
```

- [ ] `backward_slice`：从行号追溯变量定义
- [ ] `flatten_stmts`：展平函数体语句
- [ ] `analyse` 子命令（定向分析，不运行 review）

## P2 — `--llm`

```
qtcloud-code review . --llm    # 规则引擎 + LLM 分析
```

- [ ] LLM 客户端 + `--llm` 标志
- [ ] LLM 分析 finding（优先级排序、误报标记、自然语言解释）
- [ ] 置信度标注（证据锚定率计算）

## 非目标

- 不做框架、不做编排、不做「智能体」
- 不做跨 finding 根因分析（那是人的工作，工具提供证据就够了）

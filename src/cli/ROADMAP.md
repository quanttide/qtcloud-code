# ROADMAP — qtcloud-code-cli

## v0.1.0（当前）

CLI 骨架 + 硬编码检测器，规则阈值写死在代码里。

```yaml
# 当前检测器注册硬编码在 main.rs:
list_detectors() -> vec![
    UnsafeBlockDetector,    # 阈值 3/5/8
    LongFunctionDetector,   # 阈值 30/50/80
]
```

**局限**：规则不可配置，阈值不可调，不支持按项目定制。

---

## v0.2.0 — 围绕 Contract 重构

### 核心思路

用 `qtcloud-code.toml`（Contract 文件）替代硬编码规则注册和阈值。工具以 Contract 为中心：

```
Contract 文件 (.qtcloud-code.toml)
        │
        ├── 定义规则（哪些开/关）
        ├── 定义阈值（30/50 还是 60/100）
        ├── 定义忽略模式
        └── 定义语言解析器启用列表
                │
                ▼
        工具读取 Contract → 按 Contract 配置运行
```

### Contract 文件格式（初稿）

```toml
version = "1"

[languages]
rust = true
python = true
go = false
typescript = false

[rules.long-function]
enabled = true
severity = "should"
threshold = 50

[rules.unsafe-block]
enabled = true
severity = "must"
threshold = 3

[ignore]
paths = ["vendor/", "generated/"]
```

### 待实施

- [ ] 定义 `Contract` 数据模型（Rust struct + serde Deserialize）
- [ ] `config.rs` — 读取 `qtcloud-code.toml`，合并默认值，支持逐级查找（项目 → 用户 → 内置默认）
- [ ] `rule_registry.rs` — 由 Contract 驱动规则注册，替代 `list_detectors()` 硬编码
- [ ] `LangParser` 工厂方法改为由 Contract 控制启用哪些语言
- [ ] `review` 命令新增 `--contract <path>` 参数，默认查找项目根目录
- [ ] `list-rules` 改为读取 Contract 后输出：仅输出已启用的规则，附带当前阈值
- [ ] 内置默认 Contract（无 `qtcloud-code.toml` 时的后备行为，与当前硬编码行为兼容）
- [ ] `init` 子命令：`qtcloud-code init` 在当前目录生成默认 `qtcloud-code.toml`

### 兼容性

| 场景 | 行为 |
|------|------|
| 项目有 `qtcloud-code.toml` | 读取并应用，完全按 Contract 运行 |
| 项目无 `qtcloud-code.toml` | 使用内置默认 Contract（行为与 v0.1.0 一致） |
| 传 `--contract <path>` | 使用指定路径的 Contract |

不破坏向后兼容性。

---

## 未来（v0.3.0+）

- Contract 支持 version pinning（锁定规则版本）
- 多合约合并（基础合约 + 项目覆写）
- CI 集成：`qtcloud-code check --contract .qtcloud-code.toml` 失败时退出码非 0

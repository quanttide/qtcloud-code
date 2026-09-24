# reflect — 代码定向分析

定向分析，不跑 review。你指定方向，工具提供证据。

## 与 review 的关系

```
review   — 自动化扫描。输入目录，输出 finding。广度。
reflect  — 定向分析。输入文件/行/变量，输出证据链。深度。
```

`review` 帮你发现代码里的问题。  
`reflect` 帮你理解**具体的某段代码**为什么是这样、问题从哪来。

## 子命令

### slice — 反向追溯

```
reflect slice <file> <line>
```

从某行代码往回追溯：这一行的结果依赖了哪些变量。

**适用场景：** 你觉得某行代码可疑，想知道"这个值从哪来的"。

**来源：** 实验室验证——最短证据（1 行输出端）让 LLM 自主追溯全程，发现最多问题。`reflect slice` 把同样的追溯能力给人类用。

---

### trace — 变量数据流

```
reflect trace <file> <var>
reflect trace <file> <line> <var>   # 指定行号（可选）
```

追踪一个变量从声明到使用点的完整数据流。

**适用场景：** 你想知道某个变量在代码里经过了哪些转换。line 可选——不传则自动查找变量声明位置。

---

### graph — 函数级调用图

```
reflect graph <file>
```

列出文件中所有函数及其调用关系。

**适用场景：** 快速理解代码结构。

**JSON 契约（D10，证据信封形态）**：`--json` 输出为对象，`nodes` 每项是一条 `kind:"graph"` 证据信封：

| 字段 | 类型 | 说明 |
|:--|:--|:--|
| `kind` | string | 恒为 `graph` |
| `file` | string | 分析文件 |
| `line` | number | 函数定义行（1 起） |
| `text` | string | 函数名 |
| `callees` | string[] | 被调用，终末短名去重升序，单条 ≤ 80 字符 |
| `callers` | string[] | 调用方，同文件函数名去重升序 |

```json
{
  "file": "src/search.rs",
  "nodes": [
    {"kind": "graph", "file": "src/search.rs", "line": 37, "text": "search",
     "callees": ["matches"], "callers": []}
  ]
}
```

语义（D15 拆解定型）：callee 取终末方法短名（`a.b.c()` → `c`、`Foo::new()` → `new`），闭包体不作调用名；外部/标准库调用按 `audit::project_refs` 同源策略（`EXTERNAL_CALLS` 单一事实源）过滤，文件内定义的函数优先于黑名单（项目内关系不因撞名丢失）；单条限长 80 字符，防长链与多行文本撑爆契约。非 Rust 文件当前为行级函数定位，`callees`/`callers` 暂空——完整多语言节点识别登记下轮。

---

### suggest — 推荐可疑行

```
reflect suggest <file>
```

扫描文件，标记返回点、unsafe、类型转换、parse、unwrap/expect 等高风险行号，输出按风险分级排序（return 类降权殿后）。  
不做分析，只告诉你“这些行值得看”。

**适用场景：** 你不知道从哪开始分析一个文件。suggest 给你起点，然后你用 slice 继续追溯。

不替代 review——它不检测问题，只推荐候选行。

## 设计原则

- **不做自动发现**。reflect 的输入永远是人的怀疑方向。  
- **每个子命令只回答一个问题**，不跨域。拿到的结果可以换子命令继续追问。  
- **不引入 LLM 依赖**。所有分析基于 tree-sitter AST 或文本匹配，确定、可复现。  
- **和 review 互补**。review 扫广度，reflect 查深度。

# reflect 集成测试设计

## 测试策略

集成测试 = 编译后的二进制调用，用 `tempfile` 创建临时文件作为 fixture，验证 stdout/stderr 和退出码。

## 测试分类

### 1. CLI 注册（2 个）

| 测试 | 验证 |
|------|------|
| `test_reflect_help_succeeds` | `qtcloud-code reflect --help` 返回 0，输出包含子命令名 |
| `test_reflect_no_subcommand` | `qtcloud-code reflect` 返回 0/非0，输出 help 或错误提示 |

### 2. slice（4 个）

| 测试 | 输入 | 预期输出 |
|------|------|---------|
| `test_slice_basic` | 简单函数，Rust，L14 `Ok(result)` | 输出追溯链，包含 `let price = price_int as f64` 等 |
| `test_slice_empty_result` | 行号在函数体外 | 退出码 1，stderr 提示"未找到追溯结果" |
| `test_slice_nonexistent_file` | 不存在的文件 | 退出码 2，stderr 错误 |
| `test_slice_json` | 同上 basic，`--json` | stdout 是合法 JSON 数组 |

### 3. trace（4 个）

| 测试 | 输入 | 预期输出 |
|------|------|---------|
| `test_trace_basic` | 指定 line + var | 输出变量定义链 |
| `test_trace_without_line` | 只传 var，不传 line | 自动查找声明位置，输出相同结果 |
| `test_trace_nonexistent_var` | 不存在的变量 | 退出码 1，stderr 提示 |
| `test_trace_json` | `--json` | stdout 是合法 JSON 数组 |

### 4. graph（3 个）

| 测试 | 输入 | 预期输出 |
|------|------|---------|
| `test_graph_basic` | 包含多个函数的 Rust 文件 | 列出函数名和调用数 |
| `test_graph_empty` | 仅一个函数或无函数 | 退出码 1 或 0 |
| `test_graph_json` | `--json` | stdout 是合法 JSON 对象 |

### 5. suggest（3 个）

| 测试 | 输入 | 预期输出 |
|------|------|---------|
| `test_suggest_basic` | 包含 return/panic/cast 的 Rust 文件 | 输出可疑行列表 |
| `test_suggest_clean_file` | 仅 `fn main() {}` 无可疑模式 | 退出码 1，stderr 提示"未发现可疑行" |
| `test_suggest_json` | `--json` | stdout 是合法 JSON 数组 |

### 6. 多语言（3 个）

| 测试 | 验证 |
|------|------|
| `test_slice_python` | Python 文件 slice 可工作 |
| `test_trace_go` | Go 文件 trace 可工作 |
| `test_graph_typescript` | TypeScript 文件 graph 可工作 |

## Fixture 设计

所有 fixture 是内联字符串，在测试函数中用 `tempfile::tempdir()` 写入，避免外部文件依赖。

### Rust fixture（process_order）

```rust
fn process_order(input: &str) -> Result<String, String> {
    let raw = input.trim();
    let parts: Vec<&str> = raw.split(',').collect();
    let name = parts.get(0).map(|s| s.trim()).unwrap_or("?").to_string();
    let price_str = parts.get(1).map(|s| s.trim()).unwrap_or("0");
    let qty_str = parts.get(2).map(|s| s.trim()).unwrap_or("1");
    let price_int: u32 = price_str.parse().map_err(|_| "bad price")?;
    let qty: u32 = qty_str.parse().map_err(|_| "bad qty")?;
    let price = price_int as f64;
    let subtotal = price * qty as f64;
    let tax = subtotal * 0.08;
    let total = subtotal + tax;
    let result = name + ": $" + &total.to_string();
    Ok(result)
}
```

### Python fixture（slice 多语言）

```python
def process_order(input_str):
    raw = input_str.strip()
    parts = raw.split(',')
    name = parts[0].strip() if len(parts) > 0 else "?"
    price = float(parts[1].strip()) if len(parts) > 1 else 0.0
    qty = int(parts[2].strip()) if len(parts) > 2 else 1
    total = price * qty
    result = f"{name}: ${total:.2f}"
    return result
```

### Go fixture（trace 多语言）

```go
func processOrder(input string) (string, error) {
    raw := strings.TrimSpace(input)
    parts := strings.Split(raw, ",")
    name := "?"
    if len(parts) > 0 { name = strings.TrimSpace(parts[0]) }
    price := 0.0
    if len(parts) > 1 { price, _ = strconv.ParseFloat(strings.TrimSpace(parts[1]), 64) }
    qty := 1
    if len(parts) > 2 { qty, _ = strconv.Atoi(strings.TrimSpace(parts[2])) }
    total := price * float64(qty)
    result := fmt.Sprintf("%s: $%.2f", name, total)
    return result, nil
}
```

### TypeScript fixture（graph 多语言）

```typescript
function parsePrice(s: string): number { return parseFloat(s); }
function parseQty(s: string): number { return parseInt(s, 10); }
function processOrder(input: string): string {
    const parts = input.trim().split(',');
    const price = parsePrice(parts[1]);
    const qty = parseQty(parts[2]);
    return `${parts[0]}: $${(price * qty).toFixed(2)}`;
}
```

## 测试文件结构

```
tests/
├── review.rs    (已有，不改动)
└── reflect.rs   (新增，~30 个测试)
```

## Cargo.toml 变更

```toml
[[test]]
name = "reflect"
path = "tests/reflect.rs"
```

## 依赖

现有 dev-dependencies 已有 `tempfile = "3"`，无需新增。

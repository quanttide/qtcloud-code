use std::path::PathBuf;
use std::process::Command;

// ============ fixtures ============

const RUST_PROCESS_ORDER: &str = r#"fn process_order(input: &str) -> Result<String, String> {
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
"#;

const RUST_MULTI_FUNC: &str = r#"fn helper(a: i32) -> i32 { a * 2 }
fn process(x: i32) -> i32 {
    helper(x) + 1
}
fn main() {
    let _ = process(42);
}
"#;

const RUST_WITH_SUSPICIOUS: &str = r#"fn main() -> Result<(), String> {
    let x = unsafe { std::mem::transmute::<i32, u32>(42) };
    let y = "10".parse::<u32>().map_err(|e| e.to_string())?;
    let z = x as f64;
    if z > 100.0 {
        panic!("too big");
    }
    if z < 0.0 {
        return Err("negative".to_string());
    }
    Ok(())
}
"#;

const PYTHON_PROCESS_ORDER: &str = r#"def process_order(input_str):
    raw = input_str.strip()
    parts = raw.split(',')
    name = parts[0].strip() if len(parts) > 0 else "?"
    price = float(parts[1].strip()) if len(parts) > 1 else 0.0
    qty = int(parts[2].strip()) if len(parts) > 2 else 1
    total = price * qty
    result = f"{name}: ${total:.2f}"
    return result
"#;

const GO_PROCESS_ORDER: &str = r#"package main

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
"#;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qtcloud-code"))
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

fn rust_fixture(name: &str, code: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, code).unwrap();
    Fixture { _dir: dir, path }
}

// ============ CLI registration ============

#[test]
fn test_reflect_help_succeeds() {
    let output = cli().arg("reflect").arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("slice"));
    assert!(stdout.contains("trace"));
    assert!(stdout.contains("graph"));
    assert!(stdout.contains("suggest"));
}

#[test]
fn test_reflect_no_subcommand_shows_help_or_error() {
    let output = cli().arg("reflect").output().unwrap();
    let combined = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(combined.contains("slice") || combined.contains("Usage"));
}

// ============ slice ============

#[test]
fn test_slice_basic() {
    let fx = rust_fixture("test.rs", RUST_PROCESS_ORDER);
    let output = cli()
        .arg("reflect").arg("slice")
        .arg(fx.path.to_str().unwrap()).arg("14")
        .output().unwrap();
    assert!(output.status.success(), "slice failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L14") || stdout.contains("L13"), "expected trace lines, got: {}", stdout);
}

#[test]
fn test_slice_empty_result() {
    let fx = rust_fixture("empty.rs", "// just a comment\n");
    let output = cli()
        .arg("reflect").arg("slice")
        .arg(fx.path.to_str().unwrap()).arg("1")
        .output().unwrap();
    assert_eq!(output.status.code(), Some(1), "行号在函数体外应退出 1");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("未找到追溯结果"), "got: {}", stderr);
}

#[test]
fn test_slice_nonexistent_file() {
    let output = cli()
        .arg("reflect").arg("slice")
        .arg("/nonexistent/test.rs").arg("1")
        .output().unwrap();
    assert_eq!(output.status.code(), Some(2), "文件不存在应退出 2");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty(), "stderr 应有错误信息");
}

#[test]
fn test_slice_json() {
    let fx = rust_fixture("test.rs", RUST_PROCESS_ORDER);
    let output = cli()
        .arg("reflect").arg("slice")
        .arg(fx.path.to_str().unwrap()).arg("14")
        .arg("--json")
        .output().unwrap();
    assert!(output.status.success(), "slice --json failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    assert!(parsed.as_array().unwrap().len() >= 1);
    assert!(parsed[0]["line"].is_number());
    assert!(parsed[0]["text"].is_string());
}

// ============ trace ============

#[test]
fn test_trace_basic() {
    let fx = rust_fixture("test.rs", RUST_PROCESS_ORDER);
    let output = cli()
        .arg("reflect").arg("trace")
        .arg(fx.path.to_str().unwrap()).arg("price_int").arg("14")
        .output().unwrap();
    assert!(output.status.success(), "trace failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("price_int") || stdout.contains("price_str"),
        "expected trace of price_int, got: {}", stdout);
}

#[test]
fn test_trace_without_line() {
    let fx = rust_fixture("test.rs", RUST_PROCESS_ORDER);
    let output = cli()
        .arg("reflect").arg("trace")
        .arg(fx.path.to_str().unwrap()).arg("price_int")
        .output().unwrap();
    assert!(output.status.success(), "trace without line failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("price_int"), "expected trace of price_int, got: {}", stdout);
}

#[test]
fn test_trace_nonexistent_var() {
    let fx = rust_fixture("test.rs", RUST_PROCESS_ORDER);
    let output = cli()
        .arg("reflect").arg("trace")
        .arg(fx.path.to_str().unwrap()).arg("nonexistent_var_xyz")
        .output().unwrap();
    assert!(!output.status.success(), "expected non-zero exit for nonexistent var");
}

#[test]
fn test_trace_json() {
    let fx = rust_fixture("test.rs", RUST_PROCESS_ORDER);
    let output = cli()
        .arg("reflect").arg("trace")
        .arg(fx.path.to_str().unwrap()).arg("price_int").arg("14")
        .arg("--json")
        .output().unwrap();
    assert!(output.status.success(), "trace --json failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    assert!(parsed[0]["var"].is_string());
    assert!(parsed[0]["from"].is_string());
}

// ============ graph ============

#[test]
fn test_graph_basic() {
    let fx = rust_fixture("multi.rs", RUST_MULTI_FUNC);
    let output = cli()
        .arg("reflect").arg("graph")
        .arg(fx.path.to_str().unwrap())
        .output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("helper") || stdout.contains("process") || stdout.contains("main"),
        "expected function names, got: {}", stdout);
}

#[test]
fn test_graph_empty() {
    let fx = rust_fixture("empty.rs", "// just a comment\n");
    let output = cli()
        .arg("reflect").arg("graph")
        .arg(fx.path.to_str().unwrap())
        .output().unwrap();
    assert_eq!(output.status.code(), Some(1), "expected exit code 1 for empty graph");
}

#[test]
fn test_graph_json() {
    let fx = rust_fixture("multi.rs", RUST_MULTI_FUNC);
    let output = cli()
        .arg("reflect").arg("graph")
        .arg(fx.path.to_str().unwrap())
        .arg("--json")
        .output().unwrap();
    assert!(output.status.success(), "graph --json failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    assert!(parsed.as_array().unwrap().len() >= 1);
    assert!(parsed[0]["name"].is_string());
}

// ============ suggest ============

#[test]
fn test_suggest_basic() {
    let fx = rust_fixture("suspicious.rs", RUST_WITH_SUSPICIOUS);
    let output = cli()
        .arg("reflect").arg("suggest")
        .arg(fx.path.to_str().unwrap())
        .output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("return") || stdout.contains("panic") || stdout.contains("unsafe")
        || stdout.contains("cast") || stdout.contains("parse"),
        "expected suggestion categories, got: {}", stdout);
}

#[test]
fn test_suggest_clean_file() {
    let fx = rust_fixture("clean.rs", "fn main() { let x = 42; println!(\"{}\", x); }\n");
    let output = cli()
        .arg("reflect").arg("suggest")
        .arg(fx.path.to_str().unwrap())
        .output().unwrap();
    assert_eq!(output.status.code(), Some(1), "expected exit code 1 for clean file");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("未发现"), "expected 'no suspicious lines' message, got: {}", stderr);
}

#[test]
fn test_suggest_json() {
    let fx = rust_fixture("suspicious.rs", RUST_WITH_SUSPICIOUS);
    let output = cli()
        .arg("reflect").arg("suggest")
        .arg(fx.path.to_str().unwrap())
        .arg("--json")
        .output().unwrap();
    assert!(output.status.success(), "suggest --json failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    assert!(parsed[0]["kind"].is_string());
    assert!(parsed[0]["line"].is_number());
}

// ============ multi-language ============

#[test]
fn test_slice_python() {
    let fx = rust_fixture("order.py", PYTHON_PROCESS_ORDER);
    let output = cli()
        .arg("reflect").arg("slice")
        .arg(fx.path.to_str().unwrap()).arg("9")
        .output().unwrap();
    let code = output.status.code();
    assert_eq!(code, Some(0),
        "expected exit 0 for Python slice, got: {:?}", code);
}

#[test]
fn test_trace_go() {
    let fx = rust_fixture("order.go", GO_PROCESS_ORDER);
    let output = cli()
        .arg("reflect").arg("trace")
        .arg(fx.path.to_str().unwrap()).arg("price")
        .output().unwrap();
    let code = output.status.code();
    assert_eq!(code, Some(0),
        "expected exit 0 for Go trace, got: {:?} — stderr: {}", code, String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("price"), "expected trace of price, got: {}", stdout);
}

#[test]
fn test_graph_typescript() {
    let fx = rust_fixture("order.ts", r#"
function parsePrice(s: string): number { return parseFloat(s); }
function parseQty(s: string): number { return parseInt(s, 10); }
function processOrder(input: string): string {
    const parts = input.trim().split(',');
    const price = parsePrice(parts[1]);
    const qty = parseQty(parts[2]);
    return `${parts[0]}: $${(price * qty).toFixed(2)}`;
}
"#);
    let output = cli()
        .arg("reflect").arg("graph")
        .arg(fx.path.to_str().unwrap())
        .output().unwrap();
    let code = output.status.code();
    assert_eq!(code, Some(0),
        "expected exit 0 for TS graph, got: {:?}", code);
}

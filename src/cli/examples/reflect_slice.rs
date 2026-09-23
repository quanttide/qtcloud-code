//! reflect_slice — 反向程序切片：从目标行回溯影响它的语句
//!
//! 运行：`cargo run --example reflect_slice`

use std::path::Path;

const SAMPLE: &str = r#"fn process_order(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    let parts: Vec<&str> = trimmed.split(',').collect();
    let name = parts[0].trim();
    let price: f64 = parts[1].trim().parse().map_err(|e| format!("bad price: {}", e))?;
    let qty: f64 = parts[2].trim().parse().map_err(|e| format!("bad qty: {}", e))?;
    let total = price * qty;
    Ok(format!("{}: {:.2}", name, total))
}"#;

/// 素材优先：`assets/fixtures/process_order.rs`；缺省回落内嵌样例
fn load_sample() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fixtures/process_order.rs");
    std::fs::read_to_string(path).unwrap_or_else(|_| SAMPLE.to_string())
}

fn main() {
    let code = load_sample();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("加载 Rust 语法");
    let tree = parser.parse(&code, None).expect("解析示例");

    let target_line = 7; // let total = price * qty;
    let entries = qtcloud_code_cli::reflect::backward_slice(
        &code,
        &tree,
        Path::new("sample.rs"),
        target_line,
    );

    if entries.is_empty() {
        println!("目标行 L{} 无可追溯语句", target_line);
        return;
    }
    println!(
        "== 反向切片：L{} 的上游语句（共 {} 条）==",
        target_line,
        entries.len()
    );
    for e in &entries {
        println!("L{:02} {}", e.line, e.text);
    }
}

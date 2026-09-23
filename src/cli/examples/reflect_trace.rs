//! reflect_trace — 变量数据流追踪：从声明回溯 RHS 上游变量
//!
//! 运行：`cargo run --example reflect_trace`

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

    let var = "total";
    let flow = qtcloud_code_cli::reflect::trace_variable(&code, &tree, 7, var);

    if flow.is_empty() {
        println!("未找到变量 '{}' 的声明", var);
        return;
    }
    println!("== 数据流：{} 的上游链（共 {} 步）==", var, flow.len());
    for f in &flow {
        if f.from.is_empty() {
            println!("L{:02} {} = (参数或外部定义)", f.line, f.var);
        } else {
            println!("L{:02} {} = {}", f.line, f.var, f.from);
        }
    }
}

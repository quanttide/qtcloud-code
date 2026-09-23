//! reflect_graph — 函数级调用图：真实的 callees / callers 关系
//!
//! 运行：`cargo run --example reflect_graph`

use std::path::Path;

const SAMPLE: &str = r#"fn helper(a: i32) -> i32 { a * 2 }
fn process(x: i32) -> i32 {
    helper(x) + 1
}
fn main() {
    let _ = process(42);
}
"#;

/// 素材优先：`assets/fixtures/call_chain.rs`；缺省回落内嵌样例
fn load_sample() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fixtures/call_chain.rs");
    std::fs::read_to_string(path).unwrap_or_else(|_| SAMPLE.to_string())
}

fn main() {
    let code = load_sample();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("加载 Rust 语法");
    let tree = parser.parse(&code, None).expect("解析示例");

    let graph = qtcloud_code_cli::reflect::build_call_graph(&code, &tree);

    let mut nodes: Vec<_> = graph.into_values().collect();
    nodes.sort_by_key(|n| n.line);

    println!("== 调用图（共 {} 个函数）==", nodes.len());
    for n in &nodes {
        println!(
            "L{:04} {} — 调用: {:?}，被调用: {:?}",
            n.line, n.name, n.callees, n.callers
        );
    }
}

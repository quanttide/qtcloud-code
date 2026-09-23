//! reflect_graph — 函数级调用图：真实的 callees / callers 关系
//!
//! 运行：`cargo run --example reflect_graph`

const SAMPLE: &str = r#"fn helper(a: i32) -> i32 { a * 2 }
fn process(x: i32) -> i32 {
    helper(x) + 1
}
fn main() {
    let _ = process(42);
}
"#;

fn main() {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("加载 Rust 语法");
    let tree = parser.parse(SAMPLE, None).expect("解析示例");

    let graph = qtcloud_code_cli::reflect::build_call_graph(SAMPLE, &tree);

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

//! reflect_suggest — 可疑行推荐：文本启发式匹配 return / panic / unsafe / cast / parse
//!
//! 运行：`cargo run --example reflect_suggest`

use std::path::Path;

const SAMPLE: &str = r#"fn parse_age(input: &str) -> Result<f64, String> {
    let age: f64 = input.trim().parse().map_err(|e| e.to_string())?;
    if age > 150.0 {
        panic!("age out of range");
    }
    let raw = 42i32;
    let as_f64 = raw as f64;
    let _dangling = unsafe { std::ptr::read(&raw as *const i32) };
    if input.is_empty() {
        return Err("invalid".to_string());
    }
    Ok(age)
}"#;

/// 素材优先：`assets/fixtures/parse_age.rs`；缺省回落内嵌样例
fn load_sample() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fixtures/parse_age.rs");
    std::fs::read_to_string(path).unwrap_or_else(|_| SAMPLE.to_string())
}

fn main() {
    let code = load_sample();
    let hits = qtcloud_code_cli::reflect::suggest(&code);

    if hits.is_empty() {
        println!("未发现可疑行");
        return;
    }
    println!("== 可疑行（共 {} 条）==", hits.len());
    for s in &hits {
        println!("L{:02} [{}] {}", s.line, s.kind, s.text);
    }
}

//! reflect_slice — 反向程序切片：从目标行回溯影响它的语句
//!
//! 素材：自 qtcloud-work 抽取的 `as_material`（见 `assets/fixtures/`）。
//! 运行：`cargo run --example reflect_slice`

use std::path::Path;

const SAMPLE: &str = r##"// 自 qtcloud-work 抽取的案例（原 src/material/mod.rs 的 PROSE、Material 与 as_material）

const PROSE: [&str; 3] = ["md", "txt", "rst"];

#[derive(Debug, Clone)]
pub struct Material {
    pub r#type: String,
    pub content: String,
    pub source: String,
    pub created_at: String,
    pub stage: String,
}

pub fn as_material(root: &Path, path: &Path) -> Material {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let text = if PROSE.contains(&extension.as_str()) {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        String::new()
    };
    let body = if let Some(rest) = text.strip_prefix("# ") {
        rest.split_once('\n')
            .map(|(_, body)| body)
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        text.trim().to_string()
    };
    let snippet: String = body.chars().take(40).collect();
    let content = if body.chars().count() > 40 {
        format!("{}…", snippet.replace('\n', " "))
    } else {
        snippet.replace('\n', " ")
    };
    Material {
        r#type: extension,
        content,
        source: format!(
            "{}/{}",
            path.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        ),
        created_at: first_seen(path),
        stage: stage_of(root, path),
    }
}
"##;

/// 素材优先：`assets/fixtures/as_material.rs`；缺省回落内嵌样例
fn load_sample() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fixtures/as_material.rs");
    std::fs::read_to_string(path).unwrap_or_else(|_| SAMPLE.to_string())
}

fn main() {
    let code = load_sample();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("加载 Rust 语法");
    let tree = parser.parse(&code, None).expect("解析示例");

    // 目标：`let content` 的声明行——素材变更后自定位，不写死行号
    let target_line = code
        .lines()
        .position(|l| l.trim_start().starts_with("let content"))
        .map(|i| i + 1)
        .unwrap_or(1);
    let entries = qtcloud_code_cli::reflect::backward_slice(
        &code,
        &tree,
        Path::new("as_material.rs"),
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

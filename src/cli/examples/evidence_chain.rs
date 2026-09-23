//! evidence_chain — 证据链对照实验：正向链 vs 反向链对 LLM 审查结果的影响
//!
//! 由实验室 `chain_exp` 改写：Vault 取密钥改为环境变量 `QTTCODE_LLM_API_KEY`，
//! 不引入 Vault，未配置时跳过。
//!
//! 运行：`QTTCODE_LLM_API_KEY=sk-... cargo run --example evidence_chain`

use std::path::Path;

const CODE: &str = r#"fn process_order(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    let parts: Vec<&str> = trimmed.split(',').collect();
    let name = parts[0].trim();
    let price: f64 = parts[1].trim().parse().map_err(|e| format!("bad price: {}", e))?;
    let qty: f64 = parts[2].trim().parse().map_err(|e| format!("bad qty: {}", e))?;
    let total = price * qty;
    Ok(format!("{}: {:.2}", name, total))
}"#;

const SYSTEM: &str = "你是一个专业的代码审查助手，回答格式为 JSON。";

fn main() {
    let api_key = match qtcloud_code_cli::llm::get_api_key() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("未配置 QTTCODE_LLM_API_KEY，跳过 LLM 对照实验");
            return;
        }
    };

    // 解析示例代码
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("加载 Rust 语法");
    let tree = parser.parse(CODE, None).expect("解析示例");

    // L6（qty 解析）的反向切片
    let slice = qtcloud_code_cli::reflect::backward_slice(CODE, &tree, Path::new("sample.rs"), 6);
    eprintln!("=== 反向切片（共 {} 条）===", slice.len());
    for s in &slice {
        eprintln!("  L{}: {}", s.line, s.text);
    }

    let fwd_text = chain_text(&slice, false);
    let rev_text = chain_text(&slice, true);

    let fwd_prompt = format!(
        "你是一个代码审查助手。分析以下代码中从输入到输出的完整数据流，找出潜在问题。\n\n\
        代码：\n```rust\n{}\n```\n\n\
        以下是按执行顺序整理的数据流追溯链：\n```\n{}\n```\n\n\
        请输出 JSON：\
        {{\"issues\": [{{\"line\": 数字, \"severity\": \"high/medium/low\", \"description\": \"中文描述\"}}], \
        \"summary\": \"总体评价（中文）\"}}",
        CODE, fwd_text
    );
    let rev_prompt = format!(
        "你是一个代码审查助手。分析以下代码中从输出到输入的反向追溯链，找出潜在问题。\n\n\
        代码：\n```rust\n{}\n```\n\n\
        以下是从输出反向追溯到输入的数据流链：\n```\n{}\n```\n\n\
        请输出 JSON：\
        {{\"issues\": [{{\"line\": 数字, \"severity\": \"high/medium/low\", \"description\": \"中文描述\"}}], \
        \"summary\": \"总体评价（中文）\"}}",
        CODE, rev_text
    );

    let fwd_result = qtcloud_code_cli::llm::call_llm(SYSTEM, &fwd_prompt, &api_key)
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
    let rev_result = qtcloud_code_cli::llm::call_llm(SYSTEM, &rev_prompt, &api_key)
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));

    println!("===== 正向链 =====");
    println!("{}", fwd_result);
    println!("\n===== 反向链 =====");
    println!("{}", rev_result);

    println!("\n===== 对比 =====");
    println!(
        "正向链发现 {} 个问题，反向链发现 {} 个问题",
        count_issues(&fwd_result),
        count_issues(&rev_result)
    );
}

/// 拼接追溯链：`reverse = true` 时按行号降序（反向链）
fn chain_text(slice: &[qtcloud_code_cli::reflect::SliceEntry], reverse: bool) -> String {
    let mut entries: Vec<_> = slice.to_vec();
    if reverse {
        entries.sort_by_key(|s| std::cmp::Reverse(s.line));
    } else {
        entries.sort_by_key(|s| s.line);
    }
    entries
        .iter()
        .map(|s| format!("L{} {}", s.line, s.text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn count_issues(json: &str) -> usize {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
        v["issues"].as_array().map(|a| a.len()).unwrap_or(0)
    } else {
        0
    }
}

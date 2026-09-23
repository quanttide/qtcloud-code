//! evidence — 证据两步：取链喂给模型，再数结论引用了多少证据
//!
//! 合并原 `evidence_chain` 与 `evidence_count` 两个实验：
//!
//! 1. **证据计数**（确定性，零 LLM）——`count_evidence` 统计结论文本中的行号
//!    引用（L3 / 行 / line）与领域变量名，`anchor_level` 按合计分
//!    `anchored` / `partial` / `unanchored`；
//! 2. **证据链实验**（需要 LLM）——对样例取反向切片，得到同一组证据的两种排序
//!    （正向 = 执行顺序、反向 = 追溯顺序），分别喂给 LLM 对比两边发现，并用
//!    `count_evidence` 给两份输出打锚定分级。
//!
//! 计数不是 confidence：既不检查证据是否支持主张，也没有校准；与生产
//! `src/llm.rs` 的 `confidence` 字段（LLM 自评 confirm/dismiss）是两回事。
//! 计数器自实验室 `llm.rs` 的 `compute_confidence` 内联迁入（实验室保留原名），
//! 变量名清单取自下方样例词汇；子串匹配偏松（`L`、`v` 命中任意位置），
//! 归属层（reflect / 公共分析模块 / example）待定（见 ROADMAP 收尾）。
//!
//! 运行：`LLM_API_KEY=sk-... cargo run --example evidence`
//! 未配置 `LLM_API_KEY` 时只演示证据计数，链实验跳过。

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

const SYSTEM: &str = "你是一个专业的代码审查助手，回答格式为 JSON。";

/// 结论样本：覆盖计数分级全路径
const CONCLUSIONS: [&str; 4] = [
    "L3 的 price 解析和 L7 的 qty 解析用了相同模式", // 行号 2 + 变量 2 = 4 → anchored
    "L8 的数组访问未检查长度",                       // 行号 1 + 变量 0 = 1 → partial
    "职责不够单一，建议架构分层",                    // 行号 0 + 变量 0 = 0 → unanchored（纯观点）
    "parts、price、total 三处都没有做长度校验",      // 行号 0 + 变量 3 = 3 → anchored（恰过阈值）
];

fn main() {
    // 1. 证据计数：确定性，零 LLM
    println!("===== 证据计数（count_evidence + anchor_level）=====");
    for s in CONCLUSIONS {
        let ev = count_evidence(s);
        println!(
            "[{:>10}] 证据 {}（行号 {} + 变量 {}）  {}",
            anchor_level(ev),
            ev.total,
            ev.line_refs,
            ev.var_refs,
            s
        );
    }

    // 2. 证据链实验：需要 LLM
    println!("\n===== 证据链实验（正向 vs 反向）=====");
    let api_key = match qtcloud_code_cli::llm::get_api_key() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("未配置 LLM_API_KEY，跳过证据链实验（上方计数演示不受影响）");
            return;
        }
    };

    // 解析样例，取 L6（qty 解析）的反向切片作为证据集
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("加载 Rust 语法");
    let tree = parser.parse(SAMPLE, None).expect("解析样例");
    let slice = qtcloud_code_cli::reflect::backward_slice(SAMPLE, &tree, Path::new("sample.rs"), 6);
    eprintln!("=== 反向切片（共 {} 条）===", slice.len());
    for s in &slice {
        eprintln!("  L{}: {}", s.line, s.text);
    }

    let fwd_prompt = build_prompt(
        "从输入到输出的完整数据流",
        "按执行顺序整理的数据流追溯链",
        &chain_text(&slice, false),
    );
    let rev_prompt = build_prompt(
        "从输出到输入的反向追溯链",
        "从输出反向追溯到输入的数据流链",
        &chain_text(&slice, true),
    );

    let fwd_result = qtcloud_code_cli::llm::call_llm(SYSTEM, &fwd_prompt, &api_key)
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
    let rev_result = qtcloud_code_cli::llm::call_llm(SYSTEM, &rev_prompt, &api_key)
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));

    println!("\n----- 正向链 -----");
    println!("{}", fwd_result);
    println!("\n----- 反向链 -----");
    println!("{}", rev_result);

    // 同一套证据计数器给两份 LLM 输出打分——恢复实验室 chain_exp 的原始对照
    println!("\n===== 对比 =====");
    println!(
        "正向链：{} 个问题，证据分级 [{}]",
        count_issues(&fwd_result),
        anchor_level(count_evidence(&fwd_result))
    );
    println!(
        "反向链：{} 个问题，证据分级 [{}]",
        count_issues(&rev_result),
        anchor_level(count_evidence(&rev_result))
    );
}

/// 拼审查 prompt：`goal` 为分析视角，`label` 为链的说明，`chain` 为证据链正文
fn build_prompt(goal: &str, label: &str, chain: &str) -> String {
    format!(
        "你是一个代码审查助手。分析以下代码中{goal}，找出潜在问题。\n\n\
        代码：\n```rust\n{}\n```\n\n\
        以下是{label}：\n```\n{}\n```\n\n\
        请输出 JSON：\
        {{\"issues\": [{{\"line\": 数字, \"severity\": \"high/medium/low\", \"description\": \"中文描述\"}}], \
        \"summary\": \"总体评价（中文）\"}}",
        SAMPLE, chain
    )
}

/// 拼接证据链：`reverse = true` 时按行号降序（反向链）
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

/// 行号引用模式：结论中出现 L3、行、line 均计一条证据
const LINE_PATTERNS: &[&str] = &["L", "行", "line"];

/// 领域变量名模式：取自 process_order 样例代码的命名词汇
const VAR_PATTERNS: &[&str] = &[
    "parts",
    "price",
    "qty",
    "trim",
    "parse",
    "total",
    "sum",
    "v",
    "name",
    "threshold",
    "items",
    "item",
];

/// 证据计数结果
#[derive(Debug, Clone, Copy)]
struct EvidenceCount {
    line_refs: usize,
    var_refs: usize,
    total: usize,
}

/// 机制：证据发现与计数——统计文本中的行号引用与领域变量名（确定性，零 LLM）
fn count_evidence(text: &str) -> EvidenceCount {
    let line_refs = count_pattern(text, LINE_PATTERNS);
    let var_refs = count_pattern(text, VAR_PATTERNS);
    EvidenceCount {
        line_refs,
        var_refs,
        total: line_refs + var_refs,
    }
}

/// 策略：按证据合计分级——≥ 3 `anchored`，1 至 2 `partial`，0 `unanchored`
///
/// 阈值是路由规则而非概率陈述：分级反映锚定程度，不代表结论正确的置信度。
fn anchor_level(ev: EvidenceCount) -> &'static str {
    if ev.total >= 3 {
        "anchored"
    } else if ev.total >= 1 {
        "partial"
    } else {
        "unanchored"
    }
}

/// 统计 patterns 在 text 中的非重叠出现次数（子串匹配，匹配后从末尾继续）
fn count_pattern(text: &str, patterns: &[&str]) -> usize {
    let mut count = 0;
    for p in patterns {
        let mut start = 0;
        while let Some(pos) = text[start..].find(p) {
            count += 1;
            start += pos + p.len();
        }
    }
    count
}

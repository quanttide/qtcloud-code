//! confidence — 证据锚定率置信度演示：按行号/变量名引用计数分级（确定性计算，非 LLM 自评）
//!
//! `compute_confidence` 自实验室 `llm.rs` 内联迁入；与生产 `src/llm.rs` 的
//! `confidence` 字段（LLM 自评 confirm/dismiss）语义不同，归属层待语义统一后
//! 决定（见 ROADMAP 收尾）。
//!
//! 运行：`cargo run --example confidence`

fn main() {
    let samples = [
        "L3 的 price 解析和 L7 的 qty 解析用了相同模式",
        "L8 的数组访问未检查长度",
        "职责不够单一，建议架构分层",
    ];
    for s in samples {
        println!("[{:>6}] {}", compute_confidence(s), s);
    }
}

/// 计算 reflect 输出的置信度（确定性计算，非 LLM 自评）
/// 基于证据锚定率：行号引用数 + 变量名引用数
fn compute_confidence(text: &str) -> &'static str {
    let line_refs = count_pattern(text, &["L", "行", "line"]);
    let var_refs = count_pattern(
        text,
        &[
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
        ],
    );
    let total = line_refs + var_refs;

    if total >= 3 {
        "high"
    } else if total >= 1 {
        "medium"
    } else {
        "low"
    }
}

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

//! confidence — 证据锚定率置信度演示
//!
//! 给「分析结论」打一个确定性置信分：结论文本里引用了多少条代码事实——
//! 行号引用（L3 / 行3 / line 3）与领域变量名（parts、price、total……）。
//! 引用越多，说明结论越锚定在具体代码上，而不是泛泛评论：
//!
//! - 合计 ≥ 3 → `high`——结论锚定在多处代码事实；
//! - 合计 1 至 2 → `medium`——有部分证据；
//! - 合计 0 → `low`——纯观点，无代码证据。
//!
//! 与生产 `src/llm.rs` 的 `confidence` 字段（LLM 自评 confirm/dismiss）语义
//! 不同：那是模型对自己的判断，这里是数出来的——确定性、可复现、零 LLM 调用。
//! 典型用法是给 reflect 切片结论或 LLM 审查解释降权：`low` 可直接丢弃。
//!
//! `compute_confidence` 自实验室 `llm.rs` 内联迁入，变量名清单取自
//! `process_order` 样例词汇；子串匹配偏松（`L`、`v` 会命中任意位置），
//! 演示保持实验室原样，归属层（llm / reflect / example）待语义统一后决定
//! （见 ROADMAP 收尾）。
//!
//! 运行：`cargo run --example confidence`

fn main() {
    // 四个样本覆盖全部分级路径：high → medium → low → 无行号仅靠变量达 high
    let samples = [
        "L3 的 price 解析和 L7 的 qty 解析用了相同模式", // 行号 2 + 变量 2 = 4 → high
        "L8 的数组访问未检查长度",                       // 行号 1 + 变量 0 = 1 → medium
        "职责不够单一，建议架构分层",                    // 行号 0 + 变量 0 = 0 → low（纯观点）
        "parts、price、total 三处都没有做长度校验",      // 行号 0 + 变量 3 = 3 → high（恰过阈值）
    ];

    for s in samples {
        let line_refs = count_pattern(s, LINE_PATTERNS);
        let var_refs = count_pattern(s, VAR_PATTERNS);
        println!(
            "[{:>6}] 证据 {}（行号 {} + 变量 {}）  {}",
            compute_confidence(s),
            line_refs + var_refs,
            line_refs,
            var_refs,
            s
        );
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

/// 证据锚定率分级：行号引用数 + 变量名引用数
///
/// - ≥ 3：`high`；1 至 2：`medium`；0：`low`
///
/// 确定性计算，非 LLM 自评——同一文本永远得到同一结果。
fn compute_confidence(text: &str) -> &'static str {
    let total = count_pattern(text, LINE_PATTERNS) + count_pattern(text, VAR_PATTERNS);
    if total >= 3 {
        "high"
    } else if total >= 1 {
        "medium"
    } else {
        "low"
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

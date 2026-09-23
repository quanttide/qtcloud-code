//! evidence_count — 证据发现与计数演示（证据锚定分级）
//!
//! 机制：扫描结论文本，数出两类「代码事实」的出现次数——行号引用
//! （L3 / 行3 / line 3）与领域变量名（parts、price、total……）。
//! 这是确定性的证据发现与计数：不调 LLM、不判断主张是否成立，只回答
//! 「这段话引用了多少条代码事实」。
//!
//! 策略：按证据合计分三级——
//!
//! - ≥ 3 → `anchored`——结论充分锚定在代码事实上；
//! - 1 至 2 → `partial`——部分锚定；
//! - 0 → `unanchored`——未锚定，纯观点，可直接丢弃或降权。
//!
//! 它不是 confidence：既不检查证据是否支持主张，也没有校准；与生产
//! `src/llm.rs` 的 `confidence` 字段（LLM 自评 confirm/dismiss）是两回事。
//! 典型用法是给 reflect 切片结论或 LLM 审查解释做过滤降权。
//!
//! 计数器自实验室 `llm.rs` 的 `compute_confidence` 内联迁入（实验室保留原名），
//! 变量名清单取自 `process_order` 样例词汇；子串匹配偏松（`L`、`v` 会命中
//! 任意位置），演示保持实验室原样，归属层（reflect / 公共分析模块 / example）
//! 待定（见 ROADMAP 收尾）。
//!
//! 运行：`cargo run --example evidence_count`

fn main() {
    // 四个样本覆盖全部分级路径：anchored → partial → unanchored → 无行号仅靠变量达 anchored
    let samples = [
        "L3 的 price 解析和 L7 的 qty 解析用了相同模式", // 行号 2 + 变量 2 = 4 → anchored
        "L8 的数组访问未检查长度",                       // 行号 1 + 变量 0 = 1 → partial
        "职责不够单一，建议架构分层", // 行号 0 + 变量 0 = 0 → unanchored（纯观点）
        "parts、price、total 三处都没有做长度校验", // 行号 0 + 变量 3 = 3 → anchored（恰过阈值）
    ];

    for s in samples {
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

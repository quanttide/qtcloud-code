//! 证据主线的落点：证据信封、证据链与评证（对齐 `quanttide-audit-toolkit` 的 `AuditEvidence`）
//!
//! - [`CodeEvidence`]：reflect 六个输出结构体统一转入的信封——`kind`/`file`/`line`/`text`
//!   打底，按 kind 携结构化负载。Rust 侧是内部标签枚举，序列化即契约形态；
//! - [`CodeEvidenceChain`]：同一组证据的有序组织（正向/反向）+ 来源与目标，`graph` 的
//!   JSON 契约（D10）取此形态；
//! - [`count_evidence`] / [`anchor_level`]：证据发现与分级——合计 ≥ 3 `anchored`、
//!   1 至 2 `partial`、0 `unanchored`。分级是路由规则不是置信度，与 `llm.rs` 的
//!   `confidence`（LLM 自评 confirm/dismiss）分名而治。
//!
//! 计数器自实验室 `llm.rs` 的 `compute_confidence` 内联迁入（实验室原名留档），
//! 变量名清单取自 `process_order` 样例词汇；子串匹配偏松（`L`、`v` 命中任意位置）。
//!
//! 归属层就此落定（证据主线，ROADMAP 阶段二）。边界：信封是未判定素材；源结构没有
//! 文件字段的（Flow/Suggest/Graph/Type），`file` 留空，由证据链或接线方补齐；
//! `suggest` 的线索经 `From` 可入信封，但按证据主线边界默认不入 finding 的 `evidence[]`
//! （问题层拍板）。

use serde::Serialize;

use crate::reflect::{
    CallGraphNode, CodeSuggestion, CodeTypeInfo, FlowEntry, ImpactResult, SliceEntry,
};

/// 证据信封：reflect 输出的统一形态（内部标签 `kind` + 按 kind 的结构化负载）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CodeEvidence {
    /// 切片/搜索条目：源码语句或标识符引用
    Slice {
        file: String,
        line: usize,
        text: String,
    },
    /// 数据流步：变量从何而来
    Flow {
        file: String,
        line: usize,
        text: String,
        var: String,
        from: String,
    },
    /// 可疑行线索（indication，弱判定）
    Suggest {
        file: String,
        line: usize,
        text: String,
        suggest_kind: String,
    },
    /// 调用图节点：函数与调用边（`text` 取函数名，源码文件由链补齐）
    Graph {
        file: String,
        line: usize,
        text: String,
        callees: Vec<String>,
        callers: Vec<String>,
    },
    /// 影响分析：变更点的前向使用与所调函数
    Impact {
        file: String,
        line: usize,
        text: String,
        callees: Vec<String>,
        usages: Vec<CodeEvidence>,
    },
    /// 类型注解
    Type {
        file: String,
        line: usize,
        text: String,
        var: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        type_annotation: Option<String>,
    },
}

impl CodeEvidence {
    /// 信封 kind：`slice` / `flow` / `suggest` / `graph` / `impact` / `type`
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Slice { .. } => "slice",
            Self::Flow { .. } => "flow",
            Self::Suggest { .. } => "suggest",
            Self::Graph { .. } => "graph",
            Self::Impact { .. } => "impact",
            Self::Type { .. } => "type",
        }
    }

    /// 源文件；源结构无文件字段时为空串（由链/接线方补齐）
    pub fn file(&self) -> &str {
        match self {
            Self::Slice { file, .. }
            | Self::Flow { file, .. }
            | Self::Suggest { file, .. }
            | Self::Graph { file, .. }
            | Self::Impact { file, .. }
            | Self::Type { file, .. } => file,
        }
    }

    /// 行号（Graph/Impact 为定义行/变更行）
    pub fn line(&self) -> usize {
        match self {
            Self::Slice { line, .. }
            | Self::Flow { line, .. }
            | Self::Suggest { line, .. }
            | Self::Graph { line, .. }
            | Self::Impact { line, .. }
            | Self::Type { line, .. } => *line,
        }
    }

    /// 单行可读文本（Graph 取函数名，Type 取 `var: T`）
    pub fn text(&self) -> &str {
        match self {
            Self::Slice { text, .. }
            | Self::Flow { text, .. }
            | Self::Suggest { text, .. }
            | Self::Graph { text, .. }
            | Self::Impact { text, .. }
            | Self::Type { text, .. } => text,
        }
    }
}

impl From<SliceEntry> for CodeEvidence {
    fn from(e: SliceEntry) -> Self {
        Self::Slice {
            file: e.file,
            line: e.line,
            text: e.text,
        }
    }
}

impl From<FlowEntry> for CodeEvidence {
    fn from(e: FlowEntry) -> Self {
        Self::Flow {
            file: String::new(),
            line: e.line,
            text: format!("{} = {}", e.var, e.from),
            var: e.var,
            from: e.from,
        }
    }
}

impl From<CodeSuggestion> for CodeEvidence {
    fn from(e: CodeSuggestion) -> Self {
        Self::Suggest {
            file: String::new(),
            line: e.line,
            text: e.text,
            suggest_kind: e.kind.to_string(),
        }
    }
}

impl From<CallGraphNode> for CodeEvidence {
    fn from(n: CallGraphNode) -> Self {
        Self::Graph {
            file: String::new(),
            line: n.line,
            text: n.name,
            callees: n.callees,
            callers: n.callers,
        }
    }
}

impl From<ImpactResult> for CodeEvidence {
    fn from(r: ImpactResult) -> Self {
        let file = r
            .forward_usages
            .first()
            .map(|u| u.file.clone())
            .unwrap_or_default();
        let usages: Vec<CodeEvidence> = r
            .forward_usages
            .into_iter()
            .map(CodeEvidence::from)
            .collect();
        Self::Impact {
            file,
            line: r.def_line,
            text: r.var_name,
            callees: r.callees,
            usages,
        }
    }
}

impl From<CodeTypeInfo> for CodeEvidence {
    fn from(t: CodeTypeInfo) -> Self {
        let text = match &t.type_annotation {
            Some(a) => format!("{}: {}", t.var, a),
            None => t.var.clone(),
        };
        Self::Type {
            file: String::new(),
            line: t.line,
            text,
            var: t.var,
            type_annotation: t.type_annotation,
        }
    }
}

/// 证据链：同一组证据的有序组织 + 来源与目标（正向 = 执行顺序，反向 = 追溯顺序）
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CodeEvidenceChain {
    /// 证据集来源定位（如 `sample.rs:L6`）
    pub source: String,
    /// 这组证据要回答的问题
    pub target: String,
    /// 有序证据，顺序即链的方向
    pub items: Vec<CodeEvidence>,
}

impl CodeEvidenceChain {
    /// 反向链：同一组证据的逆序组织——问题不变，只换顺序
    pub fn reversed(&self) -> Self {
        let mut items = self.items.clone();
        items.reverse();
        Self {
            source: self.source.clone(),
            target: self.target.clone(),
            items,
        }
    }

    /// 拼接为提示词正文：每条 `L{line} {text}`，供 LLM 因果解释器与 example 复用
    pub fn to_text(&self) -> String {
        self.items
            .iter()
            .map(|e| format!("L{} {}", e.line(), e.text()))
            .collect::<Vec<_>>()
            .join("\n")
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
pub struct EvidenceCount {
    pub line_refs: usize,
    pub var_refs: usize,
    pub total: usize,
}

/// 机制：证据发现与计数——统计文本中的行号引用与领域变量名（确定性，零 LLM）
pub fn count_evidence(text: &str) -> EvidenceCount {
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
pub fn anchor_level(ev: EvidenceCount) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_evidence_anchored() {
        // 恰好 3 条（行号 0 + 变量 3）过阈值 → anchored；4 条同级
        let at = count_evidence("parts、price、total 三处都没有做长度校验");
        assert_eq!((at.line_refs, at.var_refs, at.total), (0, 3, 3));
        assert_eq!(anchor_level(at), "anchored");
        let more = count_evidence("L3 的 price 解析和 L7 的 qty 解析用了相同模式");
        assert_eq!((more.line_refs, more.var_refs, more.total), (2, 2, 4));
        assert_eq!(anchor_level(more), "anchored");
    }

    #[test]
    fn test_count_evidence_partial() {
        let ev = count_evidence("L8 的数组访问未检查长度");
        assert_eq!((ev.line_refs, ev.var_refs, ev.total), (1, 0, 1));
        assert_eq!(anchor_level(ev), "partial");
    }

    #[test]
    fn test_count_evidence_unanchored() {
        let ev = count_evidence("职责不够单一，建议架构分层");
        assert_eq!(ev.total, 0, "纯观点无行号与变量引用");
        assert_eq!(anchor_level(ev), "unanchored");
    }

    #[test]
    fn test_six_from_impls_shapes() {
        let slice = CodeEvidence::from(SliceEntry {
            file: "a.rs".into(),
            line: 1,
            text: "let a = 1;".into(),
        });
        assert_eq!(
            (slice.kind(), slice.file(), slice.line()),
            ("slice", "a.rs", 1)
        );

        let flow = CodeEvidence::from(FlowEntry {
            var: "v".into(),
            from: "w".into(),
            line: 2,
        });
        assert_eq!(flow.kind(), "flow");
        assert_eq!(flow.text(), "v = w");
        assert_eq!(flow.file(), "", "FlowEntry 无文件字段，留空待补");

        let sug = CodeEvidence::from(CodeSuggestion {
            line: 3,
            kind: "return",
            text: "return;".into(),
        });
        assert_eq!(sug.kind(), "suggest");

        let graph = CodeEvidence::from(CallGraphNode {
            name: "f".into(),
            line: 4,
            callees: vec!["g".into()],
            callers: vec![],
        });
        assert_eq!(
            (graph.kind(), graph.text(), graph.line()),
            ("graph", "f", 4)
        );

        let impact = CodeEvidence::from(ImpactResult {
            def_line: 5,
            var_name: "v".into(),
            forward_usages: vec![SliceEntry {
                file: "a.rs".into(),
                line: 6,
                text: "v".into(),
            }],
            callees: vec![],
        });
        assert_eq!(
            (impact.kind(), impact.file()),
            ("impact", "a.rs"),
            "Impact 从首个使用点取文件"
        );

        let typ = CodeEvidence::from(CodeTypeInfo {
            var: "n".into(),
            line: 7,
            type_annotation: Some("u8".into()),
        });
        assert_eq!(typ.kind(), "type");
        assert_eq!(typ.text(), "n: u8");
    }

    #[test]
    fn test_chain_reversed_and_to_text() {
        let chain = CodeEvidenceChain {
            source: "sample.rs:L6".into(),
            target: "process_order 的数据流".into(),
            items: vec![
                SliceEntry {
                    file: "sample.rs".into(),
                    line: 3,
                    text: "let y = x;".into(),
                }
                .into(),
                FlowEntry {
                    var: "y".into(),
                    from: "x".into(),
                    line: 3,
                }
                .into(),
            ],
        };
        let rev = chain.reversed();
        assert_eq!(chain.items[0].kind(), "slice");
        assert_eq!(rev.items[0].kind(), "flow", "反向链逆序同一组证据");
        assert_eq!(
            rev.source, chain.source,
            "来源与目标不变——问题不变，只换顺序"
        );
        assert_eq!(chain.to_text(), "L3 let y = x;\nL3 y = x");
        assert_eq!(rev.to_text(), "L3 y = x\nL3 let y = x;")
    }

    #[test]
    fn test_envelope_json_shape() {
        let e: CodeEvidence = SliceEntry {
            file: "a.rs".into(),
            line: 3,
            text: "let x;".into(),
        }
        .into();
        let json = serde_json::to_string(&e).expect("序列化");
        assert_eq!(
            json, r#"{"kind":"slice","file":"a.rs","line":3,"text":"let x;"}"#,
            "信封 JSON 契约形态（D10 的地基）"
        );
    }
}

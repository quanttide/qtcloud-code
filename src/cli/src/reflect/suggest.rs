use crate::reflect::CodeSuggestion;

/// 可疑行推荐：文本启发式匹配 return / panic / unsafe / cast / parse / unwrap。
///
/// 自 `main.rs` 的 `run_reflect_suggest` 迁入；实现保留文本匹配（不升级 AST，
/// 见 ROADMAP 阶段二「suggest 保留文本实现」）。词表按真实案例校准：cast 放宽到
/// 任意 `as` 类型（不再限 f64/i32）、增补 unwrap/expect、输出按风险分级排序——
/// 高（panic/unsafe/unwrap）→ 中（cast/parse）→ 低（return 降权殿后），同级保持行号升序。
pub fn suggest(source: &str) -> Vec<CodeSuggestion> {
    let mut suggestions = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let n = i + 1;
        let t = line.trim();
        if t.starts_with("Ok(")
            || t.starts_with("Err(")
            || t.starts_with("return ")
            || t.starts_with("return;")
        {
            suggestions.push(CodeSuggestion {
                line: n,
                kind: "return",
                text: t.to_string(),
            });
        } else if t.contains("panic!(") || t.contains("unreachable!(") || t.contains("todo!(") {
            suggestions.push(CodeSuggestion {
                line: n,
                kind: "panic",
                text: t.to_string(),
            });
        } else if t.contains("unsafe")
            && !t.starts_with("unsafe fn")
            && !t.starts_with("unsafe trait")
            && !t.starts_with("unsafe impl")
        {
            suggestions.push(CodeSuggestion {
                line: n,
                kind: "unsafe",
                text: t.to_string(),
            });
        } else if t.contains(" as ") || t.starts_with("as ") {
            suggestions.push(CodeSuggestion {
                line: n,
                kind: "cast",
                text: t.to_string(),
            });
        } else if t.contains(".parse()") || t.contains(".parse::<") {
            suggestions.push(CodeSuggestion {
                line: n,
                kind: "parse",
                text: t.to_string(),
            });
        } else if t.contains(".unwrap()") || t.contains(".expect(") {
            suggestions.push(CodeSuggestion {
                line: n,
                kind: "unwrap",
                text: t.to_string(),
            });
        }
    }
    suggestions.sort_by_key(|s| rank(s.kind));
    suggestions
}

/// 风险分级：高（panic/unsafe/unwrap）→ 中（cast/parse）→ 低（return，降权殿后）
fn rank(kind: &str) -> u8 {
    match kind {
        "panic" | "unsafe" | "unwrap" => 0,
        "cast" | "parse" => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_empty_source() {
        assert!(suggest("").is_empty());
    }

    #[test]
    fn test_suggest_kinds() {
        let source = "fn f() {\n    let x: f64 = \"1\".parse().unwrap();\n    let y = x as f64;\n    return;\n    panic!(\"boom\");\n}";
        let hits = suggest(source);
        assert!(hits.iter().any(|s| s.kind == "parse"));
        assert!(hits.iter().any(|s| s.kind == "cast"));
        assert!(hits.iter().any(|s| s.kind == "return"));
        assert!(hits.iter().any(|s| s.kind == "panic"));
    }

    #[test]
    fn test_suggest_skips_unsafe_declarations() {
        let source = "unsafe fn guard() {}\nunsafe trait T {}\nunsafe impl T for u8 {}\nlet v = unsafe { x };";
        let hits = suggest(source);
        assert_eq!(hits.len(), 1, "仅 unsafe 块应命中，声明不应命中");
        assert_eq!(hits[0].kind, "unsafe");
    }

    #[test]
    fn test_suggest_unwrap_expect() {
        let source =
            "fn f(x: Option<i32>) {\n    let a = x.unwrap();\n    let b = x.expect(\"msg\");\n}";
        let hits = suggest(source);
        assert_eq!(
            hits.iter().filter(|s| s.kind == "unwrap").count(),
            2,
            "unwrap/expect 归 unwrap 类"
        );
    }

    #[test]
    fn test_suggest_cast_any_type() {
        let source = "fn f(v: i64) {\n    let n = v as usize;\n    let m = v as u64;\n}";
        let hits = suggest(source);
        assert_eq!(
            hits.iter().filter(|s| s.kind == "cast").count(),
            2,
            "任意 as 类型都命中，不再限 f64/i32"
        );
    }

    #[test]
    fn test_suggest_return_demoted_by_class() {
        let source = "fn f() {\n    return;\n    let v = \"x\".parse().unwrap();\n    let y = data.unwrap();\n    panic!(\"bad\");\n}";
        let hits = suggest(source);
        let kinds: Vec<&str> = hits.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec!["unwrap", "panic", "parse", "return"],
            "高风险类在前、return 降权殿后，同级保持行号升序"
        );
    }
}

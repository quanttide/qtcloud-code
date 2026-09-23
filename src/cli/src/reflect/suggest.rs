use crate::reflect::CodeSuggestion;

/// 可疑行推荐：文本启发式匹配 return / panic / unsafe / cast / parse。
///
/// 自 `main.rs` 的 `run_reflect_suggest` 迁入，实现保持文本匹配（不升级 AST，
/// 见 ROADMAP 阶段二「suggest 保留文本实现」）。
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
        } else if (t.contains("as ") && t.contains("f64"))
            || (t.contains("as ") && t.contains("i32"))
        {
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
        }
    }
    suggestions
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
}

use std::path::PathBuf;

use super::{Detector, Finding, Severity};

const MAY_THRESHOLD: usize = 3;
const SHOULD_THRESHOLD: usize = 5;
const MUST_THRESHOLD: usize = 8;

pub struct UnsafeBlockDetector;

impl Detector for UnsafeBlockDetector {
    fn rule_id(&self) -> &'static str {
        "rust-wide-unsafe"
    }

    fn description(&self) -> &'static str {
        "unsafe 块包含过多语句"
    }

    fn detect(&self, _source: &str, tree: &tree_sitter::Tree, file_path: &PathBuf) -> Vec<Finding> {
        let mut findings = Vec::new();
        super::walk_tree(tree, |node| {
            if node.kind() == "unsafe_block" {
                let stmt_count = count_block_statements(&node);
                if let Some(severity) = classify(stmt_count) {
                    let pos = node.start_position();
                    findings.push(Finding {
                        file_path: file_path.clone(),
                        line: pos.row + 1,
                        column: pos.column + 1,
                        severity,
                        rule_id: self.rule_id().to_string(),
                        message: format!("unsafe 块包含 {} 条语句", stmt_count),
                    });
                }
            }
        });
        findings
    }
}

fn count_block_statements(node: &tree_sitter::Node) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return 0;
    }
    loop {
        let child = cursor.node();
        if child.kind() == "block" {
            let mut cc = child.walk();
            if cc.goto_first_child() {
                loop {
                    let stmt = cc.node();
                    if stmt.kind().ends_with("_statement") || stmt.kind() == "expression_statement" {
                        count += 1;
                    }
                    if !cc.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    count
}

fn classify(stmts: usize) -> Option<Severity> {
    if stmts > MUST_THRESHOLD {
        Some(Severity::Must)
    } else if stmts > SHOULD_THRESHOLD {
        Some(Severity::Should)
    } else if stmts > MAY_THRESHOLD {
        Some(Severity::May)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_rust_tree(source: &str) -> (String, tree_sitter::Tree) {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        (source.to_string(), tree)
    }

    #[test]
    fn test_small_unsafe_no_finding() {
        let (source, tree) = make_rust_tree("unsafe { f(1); }");
        let findings = UnsafeBlockDetector.detect(&source, &tree, &PathBuf::from("f.rs"));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wide_unsafe_should() {
        let stmts = (0..6).map(|i| format!("  f({});", i)).collect::<Vec<_>>().join("\n");
        let source = format!("unsafe {{\n{}\n}}", stmts);
        let (s, tree) = make_rust_tree(&source);
        let findings = UnsafeBlockDetector.detect(&s, &tree, &PathBuf::from("f.rs"));
        assert!(!findings.is_empty());
        assert_eq!(findings[0].severity, Severity::Should);
    }

    #[test]
    fn test_classify() {
        assert_eq!(classify(2), None);
        assert_eq!(classify(3), None);
        assert_eq!(classify(4), Some(Severity::May));
        assert_eq!(classify(5), Some(Severity::May));
        assert_eq!(classify(6), Some(Severity::Should));
        assert_eq!(classify(8), Some(Severity::Should));
        assert_eq!(classify(9), Some(Severity::Must));
    }
}

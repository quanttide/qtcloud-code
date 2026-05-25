use std::path::PathBuf;

use super::{Detector, Finding, Severity};

const MAY_THRESHOLD: usize = 30;
const SHOULD_THRESHOLD: usize = 50;
const MUST_THRESHOLD: usize = 80;

const FUNCTION_NODE_KINDS: &[&str] = &["function_item", "function_definition", "function_declaration", "method_declaration"];

pub struct LongFunctionDetector;

impl Detector for LongFunctionDetector {
    fn rule_id(&self) -> &'static str {
        "long-function"
    }

    fn description(&self) -> &'static str {
        "函数体过长"
    }

    fn detect(&self, source: &str, tree: &tree_sitter::Tree, file_path: &PathBuf) -> Vec<Finding> {
        let mut findings = Vec::new();
        super::walk_tree(tree, |node| {
            if FUNCTION_NODE_KINDS.contains(&node.kind()) {
                let start = node.start_position().row;
                let end = node.end_position().row;
                let body_lines = end - start;

                if let Some(severity) = classify(body_lines) {
                    let name = extract_function_name(&node, source);
                    findings.push(Finding {
                        file_path: file_path.clone(),
                        line: start + 1,
                        column: 1,
                        severity,
                        rule_id: self.rule_id().to_string(),
                        message: format!("函数 `{}` 共 {} 行", name, body_lines),
                    });
                }
            }
        });
        findings
    }
}

fn classify(lines: usize) -> Option<Severity> {
    if lines > MUST_THRESHOLD {
        Some(Severity::Must)
    } else if lines > SHOULD_THRESHOLD {
        Some(Severity::Should)
    } else if lines > MAY_THRESHOLD {
        Some(Severity::May)
    } else {
        None
    }
}

fn extract_function_name(node: &tree_sitter::Node, source: &str) -> String {
    if let Some(name) = find_identifier_in_children(node, source) {
        return name;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if let Some(name) = find_identifier_in_children(&child, source) {
                return name;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    "<anonymous>".to_string()
}

fn find_identifier_in_children(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "identifier" {
                if let Ok(s) = child.utf8_text(source.as_bytes()) {
                    return Some(s.to_string());
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
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
    fn test_short_function_no_finding() {
        let (source, tree) = make_rust_tree("fn f() {}");
        let findings = LongFunctionDetector.detect(&source, &tree, &PathBuf::from("f.rs"));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_long_function_may() {
        let src = (0..35).map(|i| format!("  let x{} = 1;", i)).collect::<Vec<_>>().join("\n");
        let source = format!("fn f() {{\n{}\n}}", src);
        let (s, tree) = make_rust_tree(&source);
        let findings = LongFunctionDetector.detect(&s, &tree, &PathBuf::from("f.rs"));
        assert!(!findings.is_empty());
        assert_eq!(findings[0].severity, Severity::May);
        assert_eq!(findings[0].rule_id, "long-function");
    }

    #[test]
    fn test_classify() {
        assert_eq!(classify(10), None);
        assert_eq!(classify(30), None);
        assert_eq!(classify(31), Some(Severity::May));
        assert_eq!(classify(50), Some(Severity::May));
        assert_eq!(classify(51), Some(Severity::Should));
        assert_eq!(classify(80), Some(Severity::Should));
        assert_eq!(classify(81), Some(Severity::Must));
    }

    #[test]
    fn test_dart_function_name_extracted() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_dart::LANGUAGE.into()).unwrap();
        let body: String = (0..33).map(|i| format!("  var x{} = {};", i, i)).collect::<Vec<_>>().join("\n");
        let source = format!("void long() {{\n{}\n}}", body);
        let tree = parser.parse(&source, None).unwrap();
        let findings = LongFunctionDetector.detect(&source, &tree, &PathBuf::from("f.dart"));
        assert!(!findings.is_empty());
        assert_eq!(findings[0].severity, Severity::May);
        assert!(findings[0].message.contains("long"), "Dart function name should be 'long', got: {}", findings[0].message);
    }
}

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::reflect::SliceEntry;

/// 构建反向程序切片：给定代码位置，找出所有影响该点的语句
pub fn backward_slice(
    source: &str,
    tree: &tree_sitter::Tree,
    file: &Path,
    line: usize,
) -> Vec<SliceEntry> {
    let root = tree.root_node();

    // 找到包含该行的函数
    let func = find_containing_function(&root, line);
    let Some(func) = func else { return vec![] };

    // 提取函数内所有语句（展平）
    let stmts = flatten_stmts(&func);

    // 构建声明表
    let decls = build_decls(&func, source);

    // 反向追溯
    let mut results = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = vec![line];

    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }
        let Some(stmt) = stmts.iter().find(|s| s.start_position().row + 1 == current) else {
            continue;
        };

        results.push(SliceEntry {
            file: file.to_string_lossy().to_string(),
            line: current,
            text: stmt.utf8_text(source.as_bytes()).unwrap_or("?").to_string(),
        });

        for var in extract_identifiers(stmt, source.as_bytes()) {
            if let Some(&decl_line) = decls.get(&var) {
                stack.push(decl_line);
            }
        }
    }

    results.sort_by_key(|e| e.line);
    results
}

// ===== 内部工具 =====

fn find_containing_function<'t>(
    root: &tree_sitter::Node<'t>,
    line: usize,
) -> Option<tree_sitter::Node<'t>> {
    fn search<'t>(node: &tree_sitter::Node<'t>, line: usize) -> Option<tree_sitter::Node<'t>> {
        if node.is_named() && node.kind() == "function_item" {
            let start = node.start_position().row + 1;
            let end = node.end_position().row + 1;
            if start <= line && line <= end {
                return Some(*node);
            }
        }
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if let Some(found) = search(&cursor.node(), line) {
                    return Some(found);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        None
    }
    search(root, line)
}

pub fn flatten_stmts<'t>(node: &tree_sitter::Node<'t>) -> Vec<tree_sitter::Node<'t>> {
    let mut stmts = Vec::new();
    recurse_stmt(node, &mut stmts);
    stmts
}

fn recurse_stmt<'t>(node: &tree_sitter::Node<'t>, out: &mut Vec<tree_sitter::Node<'t>>) {
    // block 的尾表达式（隐式返回，如 `Ok(result)`）不是语句节点，单独补进语句集——
    // 否则目标行落在尾表达式上时切片为空（接线验收暴露）
    let tail_id = if node.kind() == "block" && node.named_child_count() > 0 {
        Some(
            node.named_child(node.named_child_count() - 1)
                .map(|t| t.id()),
        )
    } else {
        None
    };
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.is_named() {
                let k = child.kind();
                let is_tail = tail_id.flatten() == Some(child.id()) && k != "block";
                if is_stmt(k) || is_tail {
                    out.push(child);
                }
                if is_container(k) {
                    recurse_stmt(&child, out);
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn is_stmt(k: &str) -> bool {
    matches!(
        k,
        "let_declaration" | "expression_statement" | "return_statement"
    )
}

fn is_container(k: &str) -> bool {
    matches!(
        k,
        "block"
            | "for_expression"
            | "if_expression"
            | "while_expression"
            | "loop_expression"
            | "match_expression"
            | "expression_statement"
    )
}

fn build_decls<'t>(node: &tree_sitter::Node<'t>, source: &str) -> HashMap<String, usize> {
    let mut decls = HashMap::new();
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.is_named()
                && child.kind() == "let_declaration"
                && let Some(name) = child
                    .child_by_field_name("pattern")
                    .or_else(|| {
                        // fallback: 第一个 identifier
                        let mut cc = child.walk();
                        if cc.goto_first_child() {
                            loop {
                                let sub = cc.node();
                                if sub.is_named() && sub.kind() == "identifier" {
                                    return Some(sub);
                                }
                                if !cc.goto_next_sibling() {
                                    break;
                                }
                            }
                        }
                        None
                    })
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            {
                decls.insert(name.to_string(), child.start_position().row + 1);
            }
            if child.is_named() && is_container(child.kind()) {
                for (k, v) in build_decls(&child, source) {
                    decls.entry(k).or_insert(v);
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    decls
}

fn extract_identifiers(node: &tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.is_named()
                && child.kind() == "identifier"
                && let Ok(name) = child.utf8_text(source)
            {
                names.push(name.to_string());
            }
            if child.is_named() {
                names.extend(extract_identifiers(&child, source));
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backward_slice_empty_input() {
        // 空源码和空树不应 panic
        let mut parser = tree_sitter::Parser::new();
        if parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .is_err()
        {
            return;
        }
        if let Some(tree) = parser.parse("", None) {
            let result = backward_slice("", &tree, Path::new("f.rs"), 1);
            assert!(result.is_empty());
        }
    }

    #[test]
    fn test_backward_slice_out_of_range_line() {
        let code = "fn f() { let x = 1; x }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        if let Some(tree) = parser.parse(code, None) {
            let result = backward_slice(code, &tree, Path::new("f.rs"), 999);
            assert!(result.is_empty(), "out-of-range line should return empty");
        }
    }

    #[test]
    fn test_backward_slice_basic() {
        let code = "fn f() { let x = 1; let y = x + 1; y }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        if let Some(tree) = parser.parse(code, None) {
            let result = backward_slice(code, &tree, Path::new("f.rs"), 1);
            assert!(!result.is_empty(), "should find at least the target line");
        }
    }

    #[test]
    fn test_backward_slice_traces_variable() {
        // y 依赖 x，切片应从 y 追溯到 x
        let code = "fn f() {\nlet x = 1;\nlet y = x + 1;\ny\n}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        if let Some(tree) = parser.parse(code, None) {
            // L3: let y = x + 1;
            let result = backward_slice(code, &tree, Path::new("f.rs"), 3);
            assert!(
                result.iter().any(|s| s.text.contains("let x")),
                "should trace from y back to x definition"
            );
            assert!(
                result.iter().any(|s| s.text.contains("let y")),
                "should include the target statement"
            );
        }
    }

    #[test]
    fn test_backward_slice_multiline_function() {
        let code = "fn f(items: &[i32]) -> i32 {\nlet mut sum = 0;\nfor item in items {\nlet v = *item;\nsum += v;\n}\nsum\n}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        if let Some(tree) = parser.parse(code, None) {
            let result = backward_slice(code, &tree, Path::new("f.rs"), 5);
            assert!(
                result.iter().any(|s| s.text.contains("let mut sum")),
                "should trace sum to its definition"
            );
            assert!(
                result.iter().any(|s| s.text.contains("let v")),
                "should trace v to its definition"
            );
        }
    }

    #[test]
    fn test_backward_slice_traces_through_method_chain() {
        let code = "fn f() {\nlet raw = \"a,b,c\";\nlet parts: Vec<&str> = raw.split(',').collect();\nlet v = parts[2].trim();\n}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        if let Some(tree) = parser.parse(code, None) {
            // 从 L4 (parts[2].trim()) 追溯，应找到 L3 (parts 定义) 和 L2 (raw 定义)
            let result = backward_slice(code, &tree, Path::new("f.rs"), 4);
            assert!(
                result.iter().any(|s| s.line == 2 && s.text.contains("raw")),
                "should trace from parts[2].trim() back to raw definition; got lines: {:?}",
                result.iter().map(|s| s.line).collect::<Vec<_>>()
            );
            assert!(
                result
                    .iter()
                    .any(|s| s.line == 3 && s.text.contains("parts")),
                "should trace from parts[2].trim() back to parts definition; got lines: {:?}",
                result.iter().map(|s| s.line).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_backward_slice_on_tail_expression() {
        // 目标行是函数尾表达式（非语句节点）时切片不应为空
        let code = "fn f(x: i32) -> i32 {\n    let y = x + 1;\n    y * 2\n}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        if let Some(tree) = parser.parse(code, None) {
            let result = backward_slice(code, &tree, Path::new("f.rs"), 3);
            assert!(
                result.iter().any(|s| s.line == 3),
                "尾表达式本身应在切片内；got: {:?}",
                result.iter().map(|s| s.line).collect::<Vec<_>>()
            );
            assert!(
                result.iter().any(|s| s.text.contains("let y")),
                "尾表达式依赖应追溯到上游声明"
            );
        }
    }
}

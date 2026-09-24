use std::collections::HashMap;

use crate::reflect::SliceEntry;
use crate::walk::walk_all;

// ============ forward_slice ============

/// 正向切片：从定义点出发，找出所有使用该定义的引用
pub fn forward_slice(
    source: &str,
    tree: &tree_sitter::Tree,
    file: &str,
    def_line: usize,
) -> Vec<SliceEntry> {
    let root = tree.root_node();
    let name = extract_def_name_at_line(&root, source, def_line);
    let Some(name) = name else { return vec![] };

    let mut results = Vec::new();
    walk_all(&root, &mut |n| {
        if n.is_named()
            && n.kind() == "identifier"
            && let Ok(text) = n.utf8_text(source.as_bytes())
            && text == name
            && n.start_position().row + 1 != def_line
        {
            results.push(SliceEntry {
                file: file.to_string(),
                line: n.start_position().row + 1,
                text: n.utf8_text(source.as_bytes()).unwrap_or("?").to_string(),
            });
        }
    });
    results.sort_by_key(|e| e.line);
    results
}

fn extract_def_name_at_line(root: &tree_sitter::Node, source: &str, line: usize) -> Option<String> {
    let mut result = None;
    walk_all(root, &mut |n| {
        if result.is_some() {
            return;
        }
        if n.is_named() && n.kind() == "let_declaration" && n.start_position().row + 1 == line {
            let mut cc = n.walk();
            if cc.goto_first_child() {
                loop {
                    let child = cc.node();
                    if child.is_named() && child.kind() == "identifier" {
                        result = child
                            .utf8_text(source.as_bytes())
                            .ok()
                            .map(|s| s.to_string());
                        break;
                    }
                    if !cc.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
    });
    result
}

// ============ call_graph ============

#[derive(Debug, Clone)]
pub struct CallGraphNode {
    pub name: String,
    pub line: usize,
    pub callees: Vec<String>,
    pub callers: Vec<String>,
}

/// 构建函数级调用图
pub fn build_call_graph(source: &str, tree: &tree_sitter::Tree) -> HashMap<String, CallGraphNode> {
    let root = tree.root_node();
    let mut nodes: HashMap<String, Vec<String>> = HashMap::new();
    let mut def_lines: HashMap<String, usize> = HashMap::new();

    walk_all(&root, &mut |n| {
        if n.is_named() {
            match n.kind() {
                "function_item" => {
                    if let Some(name) = n
                        .child_by_field_name("name")
                        .and_then(|nn| nn.utf8_text(source.as_bytes()).ok())
                    {
                        def_lines
                            .entry(name.to_string())
                            .or_insert(n.start_position().row + 1);
                        nodes.entry(name.to_string()).or_default();
                    }
                }
                "call_expression" => {
                    let caller = find_containing_function_name_safe(
                        &root,
                        n.start_position().row + 1,
                        source,
                    );
                    if let Some(callee) = n
                        .child_by_field_name("function")
                        .or_else(|| {
                            let mut cc = n.walk();
                            if cc.goto_first_child() {
                                loop {
                                    let ch = cc.node();
                                    if ch.is_named() && ch.kind() == "identifier" {
                                        return Some(ch);
                                    }
                                    if !cc.goto_next_sibling() {
                                        break;
                                    }
                                }
                            }
                            None
                        })
                        .and_then(|c| c.utf8_text(source.as_bytes()).ok())
                        && let Some(caller) = caller
                    {
                        nodes.entry(caller).or_default().push(callee.to_string());
                    }
                }
                _ => {}
            }
        }
    });

    let mut graph = HashMap::new();
    for (name, callees) in &nodes {
        let callers: Vec<String> = nodes
            .iter()
            .filter(|(_, callee_list)| callee_list.contains(name))
            .map(|(caller, _)| caller.clone())
            .collect();
        graph.insert(
            name.clone(),
            CallGraphNode {
                name: name.clone(),
                line: *def_lines.get(name).unwrap_or(&0),
                callees: callees.clone(),
                callers,
            },
        );
    }
    graph
}

fn find_containing_function_name_safe(
    root: &tree_sitter::Node,
    line: usize,
    source: &str,
) -> Option<String> {
    let mut result = None;
    walk_all(root, &mut |n| {
        if result.is_some() {
            return;
        }
        if n.is_named() && n.kind() == "function_item" {
            let start = n.start_position().row + 1;
            let end = n.end_position().row + 1;
            if start <= line && line <= end {
                result = n
                    .child_by_field_name("name")
                    .and_then(|nn| nn.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());
            }
        }
    });
    result
}

// ============ impact_analysis ============

#[derive(Debug)]
pub struct ImpactResult {
    pub def_line: usize,
    pub var_name: String,
    pub forward_usages: Vec<SliceEntry>,
    pub callees: Vec<String>,
}

/// 变更影响分析：给定一行变更，找出哪些代码会受影响
pub fn impact_analysis(
    source: &str,
    tree: &tree_sitter::Tree,
    file: &str,
    line: usize,
) -> ImpactResult {
    let name = extract_def_name_at_line(&tree.root_node(), source, line).unwrap_or_else(|| {
        find_containing_function_name_safe(&tree.root_node(), line, source).unwrap_or_default()
    });

    let forward = forward_slice(source, tree, file, line);
    let graph = build_call_graph(source, tree);
    let callees = graph
        .get(&name)
        .map(|n| n.callees.clone())
        .unwrap_or_default();

    ImpactResult {
        def_line: line,
        var_name: name,
        forward_usages: forward,
        callees,
    }
}

// ============ code_search ============

/// 按节点类型搜索代码
pub fn code_search(source: &str, tree: &tree_sitter::Tree, target_kind: &str) -> Vec<SliceEntry> {
    let mut results = Vec::new();
    walk_all(&tree.root_node(), &mut |n| {
        if n.is_named() && n.kind() == target_kind {
            results.push(SliceEntry {
                file: String::new(),
                line: n.start_position().row + 1,
                text: n.utf8_text(source.as_bytes()).unwrap_or("?").to_string(),
            });
        }
    });
    results
}

// ============ type_info ============

#[derive(Debug)]
pub struct CodeTypeInfo {
    pub var: String,
    pub line: usize,
    pub type_annotation: Option<String>,
}

/// 提取变量类型注解
pub fn type_info(source: &str, tree: &tree_sitter::Tree) -> Vec<CodeTypeInfo> {
    let mut results = Vec::new();
    let root = tree.root_node();
    walk_all(&root, &mut |n| {
        if n.is_named() && n.kind() == "let_declaration" {
            let mut var = None;
            let mut typ = None;
            let mut cc = n.walk();
            if cc.goto_first_child() {
                loop {
                    let child = cc.node();
                    let kind = child.kind();
                    if child.is_named() && kind == "identifier" && var.is_none() {
                        var = child
                            .utf8_text(source.as_bytes())
                            .ok()
                            .map(|s| s.to_string());
                    }
                    // 类型注解: : Type
                    if kind == ":"
                        && let Some(next) = cc.node().next_named_sibling()
                    {
                        typ = next
                            .utf8_text(source.as_bytes())
                            .ok()
                            .map(|s| s.to_string());
                    }
                    if !cc.goto_next_sibling() {
                        break;
                    }
                }
            }
            if let Some(var) = var {
                results.push(CodeTypeInfo {
                    var,
                    line: n.start_position().row + 1,
                    type_annotation: typ,
                });
            }
        }
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "fn main() {\n    let base = helper(1);\n    let total: i32 = base + 1;\n    println!(\"{}: {}\", base, total);\n}\n\nfn helper(x: i32) -> i32 {\n    x + 1\n}";

    fn parse(code: &str) -> tree_sitter::Tree {
        let mut p = tree_sitter::Parser::new();
        p.set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("加载 Rust 语法");
        p.parse(code, None).expect("解析源码")
    }

    #[test]
    fn test_forward_slice_finds_usages_sorted() {
        let tree = parse(SAMPLE);
        let hits = forward_slice(SAMPLE, &tree, "sample.rs", 2);
        let lines: Vec<usize> = hits.iter().map(|e| e.line).collect();
        assert_eq!(lines, vec![3, 4], "base 的使用点按行号升序、且不含定义行");
        assert!(hits.iter().all(|e| e.file == "sample.rs"));
        assert!(hits.iter().all(|e| e.text == "base"));
    }

    #[test]
    fn test_forward_slice_non_definition_line_empty() {
        let tree = parse(SAMPLE);
        assert!(
            forward_slice(SAMPLE, &tree, "sample.rs", 5).is_empty(),
            "非 let 行返回空"
        );
    }

    #[test]
    fn test_call_graph_nodes_and_relations() {
        let tree = parse(SAMPLE);
        let g = build_call_graph(SAMPLE, &tree);
        let main = g.get("main").expect("main 节点存在");
        assert_eq!(main.line, 1);
        assert_eq!(main.callees, vec!["helper"], "main 调用 helper");
        assert!(main.callers.is_empty());
        let helper = g.get("helper").expect("helper 节点存在");
        assert_eq!(helper.line, 7);
        assert!(helper.callees.is_empty());
        assert_eq!(helper.callers, vec!["main"], "helper 被 main 调用");
    }

    #[test]
    fn test_impact_analysis_variable_def() {
        let tree = parse(SAMPLE);
        let imp = impact_analysis(SAMPLE, &tree, "sample.rs", 2);
        assert_eq!(imp.var_name, "base");
        assert_eq!(imp.def_line, 2);
        let lines: Vec<usize> = imp.forward_usages.iter().map(|e| e.line).collect();
        assert_eq!(lines, vec![3, 4]);
        assert!(imp.callees.is_empty(), "变量不是函数节点，callees 为空");
    }

    #[test]
    fn test_impact_analysis_function_line() {
        let tree = parse(SAMPLE);
        let imp = impact_analysis(SAMPLE, &tree, "sample.rs", 1);
        assert_eq!(imp.var_name, "main", "退化到所在函数名");
        assert!(imp.forward_usages.is_empty());
        assert_eq!(imp.callees, vec!["helper"]);
    }

    #[test]
    fn test_code_search_by_kind() {
        let tree = parse(SAMPLE);
        let calls = code_search(SAMPLE, &tree, "call_expression");
        assert_eq!(
            calls.len(),
            1,
            "样例中仅 helper(1) 是调用表达式（println! 是宏）"
        );
        assert_eq!(calls[0].line, 2);
        assert_eq!(calls[0].text, "helper(1)");

        let decls = code_search(SAMPLE, &tree, "let_declaration");
        let lines: Vec<usize> = decls.iter().map(|e| e.line).collect();
        assert_eq!(lines, vec![2, 3]);
    }

    #[test]
    fn test_type_info_annotations() {
        let tree = parse(SAMPLE);
        let info = type_info(SAMPLE, &tree);
        assert_eq!(info.len(), 2, "两条 let 声明");
        assert_eq!(info[0].var, "base");
        assert_eq!(info[0].line, 2);
        assert!(info[0].type_annotation.is_none(), "base 无注解");
        assert_eq!(info[1].var, "total");
        assert_eq!(info[1].type_annotation.as_deref(), Some("i32"));
    }
}

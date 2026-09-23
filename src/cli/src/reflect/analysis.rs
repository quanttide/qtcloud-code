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
pub struct TypeInfo {
    pub var: String,
    pub line: usize,
    pub type_annotation: Option<String>,
}

/// 提取变量类型注解
pub fn type_info(source: &str, tree: &tree_sitter::Tree) -> Vec<TypeInfo> {
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
                results.push(TypeInfo {
                    var,
                    line: n.start_position().row + 1,
                    type_annotation: typ,
                });
            }
        }
    });
    results
}

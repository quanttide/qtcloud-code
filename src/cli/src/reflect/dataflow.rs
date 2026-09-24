use std::collections::{HashMap, HashSet};

use crate::reflect::FlowEntry;
use crate::walk::walk_all;

/// 追踪变量的数据流路径：从使用点追溯到源头
///
/// 跨函数追踪（本轮新增）：RHS 中出现文件内函数名时，追入其返回/尾表达式引用的
/// 变量；实参词元由同一 RHS 的词法提取自然覆盖，参数名无声明则自然止步。
pub fn trace_variable(
    source: &str,
    tree: &tree_sitter::Tree,
    start_line: usize,
    var: &str,
) -> Vec<FlowEntry> {
    let root = tree.root_node();
    let decls = collect_all_decls(&root, source);
    let fns = collect_fn_ret_vars(&root, source);
    let mut path = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = vec![(var.to_string(), start_line)];

    while let Some((current_var, _line)) = stack.pop() {
        if !visited.insert(current_var.clone()) {
            continue;
        }

        // 找到声明行
        let decl_line = match decls.get(&current_var) {
            Some(&l) => l,
            None => continue,
        };

        // 找到声明语句
        let stmt_text = extract_stmt_at_line(&root, decl_line, source);
        let from = if let Some(ref text) = stmt_text {
            extract_rhs(text)
        } else {
            String::new()
        };

        path.push(FlowEntry {
            var: current_var.clone(),
            from: from.clone(),
            line: decl_line,
        });

        // 从 RHS 提取上游变量继续追踪；命中文件内函数名则跨函数追入返回表达式
        for upstream in extract_upstream_vars(&from) {
            if !decls.contains_key(&upstream)
                && let Some(ret_vars) = fns.get(&upstream)
            {
                for rv in ret_vars {
                    stack.push((rv.clone(), decl_line));
                }
            }
            stack.push((upstream, decl_line));
        }
    }

    path
}

/// 文件内函数名 → 返回语句与尾表达式引用的变量集（跨函数追踪的入口表）
fn collect_fn_ret_vars(root: &tree_sitter::Node, source: &str) -> HashMap<String, Vec<String>> {
    let mut fns: HashMap<String, Vec<String>> = HashMap::new();
    walk_all(root, &mut |n| {
        if !(n.is_named() && n.kind() == "function_item") {
            return;
        }
        let Some(name) = n
            .child_by_field_name("name")
            .and_then(|nn| nn.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string())
        else {
            return;
        };
        let Some(body) = n.child_by_field_name("body") else {
            return;
        };
        let mut ret_vars: Vec<String> = Vec::new();
        let mut last_child: Option<tree_sitter::Node> = None;
        let mut cur = body.walk();
        if cur.goto_first_child() {
            loop {
                let child = cur.node();
                if child.is_named() {
                    if child.kind() == "return_statement"
                        && let Ok(text) = child.utf8_text(source.as_bytes())
                    {
                        let expr = text.trim_start_matches("return").trim();
                        ret_vars.extend(extract_upstream_vars(expr));
                    }
                    last_child = Some(child);
                }
                if !cur.goto_next_sibling() {
                    break;
                }
            }
        }
        // 尾表达式（隐式返回）
        if let Some(last) = last_child
            && last.kind() != "return_statement"
            && let Ok(text) = last.utf8_text(source.as_bytes())
        {
            ret_vars.extend(extract_upstream_vars(text));
        }
        ret_vars.sort();
        ret_vars.dedup();
        if !ret_vars.is_empty() {
            fns.insert(name, ret_vars);
        }
    });
    fns
}

fn collect_all_decls(root: &tree_sitter::Node, source: &str) -> HashMap<String, usize> {
    let mut decls = HashMap::new();
    walk_all(root, &mut |n| {
        if n.is_named()
            && n.kind() == "let_declaration"
            && let Some(name) = extract_let_name(&n, source)
        {
            decls.insert(name.to_string(), n.start_position().row + 1);
        }
    });
    decls
}

fn extract_let_name<'t>(node: &tree_sitter::Node<'t>, source: &str) -> Option<String> {
    if let Some(name) = node
        .child_by_field_name("pattern")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
    {
        return Some(name.to_string());
    }
    // fallback: 第一个 identifier
    let mut result = None;
    walk_all(node, &mut |n| {
        if result.is_some() {
            return;
        }
        if n.is_named()
            && n.kind() == "identifier"
            && let Ok(name) = n.utf8_text(source.as_bytes())
        {
            result = Some(name.to_string());
        }
    });
    result
}

fn extract_stmt_at_line(root: &tree_sitter::Node, line: usize, source: &str) -> Option<String> {
    // 前序首个起始于目标行且为语句/容器类的节点（与原下降优先遍历等价，且保证终止）
    let mut result = None;
    walk_all(root, &mut |n| {
        if result.is_some() {
            return;
        }
        if n.is_named() && n.start_position().row + 1 == line && is_stmt_or_cont(n.kind()) {
            result = n.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        }
    });
    result
}

fn is_stmt_or_cont(k: &str) -> bool {
    matches!(
        k,
        "let_declaration"
            | "expression_statement"
            | "return_statement"
            | "block"
            | "for_expression"
            | "if_expression"
    )
}

fn extract_rhs(stmt: &str) -> String {
    if let Some(eq) = stmt.find('=') {
        stmt[eq + 1..].trim_end_matches(';').trim().to_string()
    } else {
        stmt.to_string()
    }
}

fn extract_upstream_vars(rhs: &str) -> Vec<String> {
    let mut vars = Vec::new();
    for word in rhs.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if !word.is_empty()
            && word.chars().all(|c| c.is_alphanumeric() || c == '_')
            && word != word.to_uppercase()
        // 跳过常量
        {
            vars.push(word.to_string());
        }
    }
    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_variable_empty() {
        // 空源码不应 panic
        let mut parser = tree_sitter::Parser::new();
        if parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .is_err()
        {
            return;
        }
        if let Some(tree) = parser.parse("", None) {
            let r = trace_variable("", &tree, 1, "x");
            assert!(r.is_empty());
        }
    }

    #[test]
    fn test_trace_variable_finds_decl() {
        let code = "fn f() {\nlet x = 1;\nlet y = x;\ny\n}";
        let mut p = tree_sitter::Parser::new();
        if p.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() {
            return;
        }
        if let Some(tree) = p.parse(code, None) {
            let r = trace_variable(code, &tree, 3, "y");
            assert!(!r.is_empty(), "should find y's declaration");
            assert!(r.iter().any(|e| e.var == "y"), "should trace y");
            // y depends on x, should also find x
            assert!(
                r.iter().any(|e| e.var == "x"),
                "should trace x as upstream of y"
            );
        }
    }

    #[test]
    fn test_trace_variable_unknown() {
        let code = "fn f() { let x = 1; x }";
        let mut p = tree_sitter::Parser::new();
        if p.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() {
            return;
        }
        if let Some(tree) = p.parse(code, None) {
            let r = trace_variable(code, &tree, 1, "nonexistent");
            assert!(r.is_empty(), "unknown var should return empty");
        }
    }

    #[test]
    fn test_trace_variable_cross_function() {
        let code = "fn helper(base: i32) -> i32 {\n    let scaled = base * 2;\n    scaled + 1\n}\nfn main() {\n    let offset = 10;\n    let total = helper(offset);\n    total\n}";
        let mut p = tree_sitter::Parser::new();
        if p.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() {
            return;
        }
        if let Some(tree) = p.parse(code, None) {
            let r = trace_variable(code, &tree, 7, "total");
            let vars: Vec<&str> = r.iter().map(|e| e.var.as_str()).collect();
            assert!(vars.contains(&"total"), "起点自身：{:?}", vars);
            assert!(vars.contains(&"offset"), "实参变量经调用点覆盖：{:?}", vars);
            assert!(
                vars.contains(&"scaled"),
                "跨函数：经 helper 返回尾表达式追入函数体：{:?}",
                vars
            );
        }
    }
}

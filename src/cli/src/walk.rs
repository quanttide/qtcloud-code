//! walk_all — 语法树深度优先遍历
//!
//! 实验室三处与 `refactor/rename.rs` 共四份重复实现收敛为本模块。

/// 深度优先遍历：先访问节点本身，再依次访问子树。
pub fn walk_all<F: FnMut(tree_sitter::Node)>(node: &tree_sitter::Node, f: &mut F) {
    f(*node);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_all(&cursor.node(), f);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walk_all_terminates() {
        // 验证 walk_all 在各种树结构上都能终止（不会无限循环）
        let cases = ["fn a() {}", "fn b() { fn c() {} }", "", "mod x; use y;"];
        for code in &cases {
            let mut parser = tree_sitter::Parser::new();
            if parser
                .set_language(&tree_sitter_rust::LANGUAGE.into())
                .is_err()
            {
                continue;
            }
            if let Some(tree) = parser.parse(code, None) {
                let root = tree.root_node();
                let mut count = 0;
                walk_all(&root, &mut |_| count += 1);
                assert!(count > 0, "walk_all should visit at least root node");
                assert!(
                    count < 1000,
                    "walk_all should not loop infinitely (visited {})",
                    count
                );
            }
        }
    }
}

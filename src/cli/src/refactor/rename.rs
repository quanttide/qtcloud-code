use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn walk_all<F: FnMut(tree_sitter::Node)>(node: &tree_sitter::Node, f: &mut F) {
    f(*node);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_all(&cursor.node(), f);
            if !cursor.goto_next_sibling() { break; }
        }
    }
}

/// 符号表：符号定义 → 所有引用位置
#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub def_file: PathBuf,
    pub def_line: usize,
    pub refs: Vec<RefLocation>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Variable,
    Module,
}

#[derive(Debug, Clone)]
pub struct RefLocation {
    pub file: PathBuf,
    pub line: usize,
}

/// 构建符号表
pub fn build_symbol_table(source: &str, tree: &tree_sitter::Tree, file: &Path) -> SymbolTable {
    let root = tree.root_node();
    let mut symbols = Vec::new();

    // 第一遍：收集函数定义
    let mut func_defs: Vec<(String, usize)> = Vec::new();
    walk_all(&root, &mut |n| {
        if n.is_named() && n.kind() == "function_item" {
            if let Some(name) = n.child_by_field_name("name")
                .and_then(|nn| nn.utf8_text(source.as_bytes()).ok())
            {
                func_defs.push((name.to_string(), n.start_position().row + 1));
            }
        }
    });

    // 第二遍：对每个函数找调用
    for (name, def_line) in &func_defs {
        let mut refs = Vec::new();
        walk_all(&root, &mut |n| {
            if n.is_named() && n.kind() == "call_expression" {
                if let Some(callee) = n.child_by_field_name("function")
                    .or_else(|| {
                        let mut cc = n.walk();
                        if cc.goto_first_child() {
                            loop {
                                let ch = cc.node();
                                if ch.is_named() && ch.kind() == "identifier" { return Some(ch); }
                                if !cc.goto_next_sibling() { break; }
                            }
                        }
                        None
                    })
                    .and_then(|nn| nn.utf8_text(source.as_bytes()).ok())
                {
                    if callee == *name {
                        refs.push(RefLocation {
                            file: file.to_path_buf(),
                            line: n.start_position().row + 1,
                        });
                    }
                }
            }
        });
        symbols.push(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            def_file: file.to_path_buf(),
            def_line: *def_line,
            refs,
        });
    }

    SymbolTable { symbols }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rename_symbol() {
        let sym = Symbol {
            name: "foo".into(),
            kind: SymbolKind::Function,
            def_file: PathBuf::from("src/lib.rs"),
            def_line: 5,
            refs: vec![
                RefLocation { file: PathBuf::from("src/main.rs"), line: 10 },
            ],
        };
        let table = SymbolTable { symbols: vec![sym] };
        let r = rename_symbol(&table, "foo", "bar");
        assert_eq!(r.len(), 2);
        assert_eq!(r.get("src/lib.rs:5").unwrap(), "bar");
        assert_eq!(r.get("src/main.rs:10").unwrap(), "bar");
    }

    #[test]
    fn test_rename_symbol_no_match() {
        let table = SymbolTable { symbols: vec![] };
        assert!(rename_symbol(&table, "foo", "bar").is_empty());
    }

    #[test]
    fn test_build_symbol_table_basic() {
        let code = "fn hello() {} fn main() { hello(); }";
        let mut p = tree_sitter::Parser::new();
        if p.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() { return; }
        if let Some(tree) = p.parse(code, None) {
            let table = build_symbol_table(code, &tree, Path::new("f.rs"));
            assert!(table.symbols.iter().any(|s| s.name == "hello"), "should find hello");
            assert!(table.symbols.iter().any(|s| s.name == "main"), "should find main");
            let hello = table.symbols.iter().find(|s| s.name == "hello").unwrap();
            assert_eq!(hello.refs.len(), 1, "hello should be called once");
        }
    }

    #[test]
    fn test_walk_all_terminates() {
        let code = "fn a() { fn b() { fn c() {} } }";
        let mut p = tree_sitter::Parser::new();
        if p.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() { return; }
        if let Some(tree) = p.parse(code, None) {
            let mut count = 0;
            walk_all(&tree.root_node(), &mut |_| { count += 1; });
            assert!(count < 500, "walk_all count: {}", count);
        }
    }
}

/// 重命名符号：生成替换映射
pub fn rename_symbol(table: &SymbolTable, old_name: &str, new_name: &str) -> HashMap<String, String> {
    let mut replacements = HashMap::new();
    for sym in &table.symbols {
        if sym.name == old_name {
            replacements.insert(format!("{}:{}", sym.def_file.display(), sym.def_line), new_name.to_string());
            for rf in &sym.refs {
                replacements.insert(format!("{}:{}", rf.file.display(), rf.line), new_name.to_string());
            }
        }
    }
    replacements
}

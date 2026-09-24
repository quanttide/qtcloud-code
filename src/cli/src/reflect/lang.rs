//! 多语言定位层：函数作用域、声明行、行级函数清单
//!
//! slice/trace/graph 接线时自 `main.rs` 移植——lab 实现仅识别 Rust 节点，
//! 直接接线会在 py/go/ts 上回归（py 探针已验证，见 dev-guide/reflect.md 已知缺陷）。
//! 本层只做「定位」，AST 语义分析仍按语言能力分层：Rust 走 `reflect::*`，
//! 其余语言的完整节点识别登记下轮（ROADMAP 下轮 backlog）。

/// 找包含目标行的函数起点行（AST，多语言节点识别：rs/py/go/ts）
///
/// 找不到（目标不在任何函数内）返回 `None`，调用方按文件头 `1` 继续，
/// 与移植前的行级收集语义一致。
pub fn find_function_start(tree: &tree_sitter::Tree, lang: &str, line: usize) -> Option<usize> {
    let cursor = &mut tree.walk();
    'search: loop {
        let node = cursor.node();
        let kind = node.kind();
        let is_function = match lang {
            "rs" => kind == "function_item",
            "py" => kind == "function_definition",
            "go" => kind == "function_declaration",
            _ => kind == "function_declaration" || kind == "function",
        };
        if is_function {
            let s = node.start_position().row + 1;
            let e = node.end_position().row + 1;
            if s <= line && line <= e {
                return Some(s);
            }
        }
        if !cursor.goto_first_child() {
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    break 'search;
                }
            }
        }
    }
    None
}

/// 多语言变量声明行识别（行级）：rs `let` / py 赋值 / go `var`、`:=`
///
/// TS 不做自动探测——`let`/`const`/`var` 同名冲突太多，误判风险大于收益（移植前即如此）。
pub fn find_decl_line(source: &str, var: &str, ext: &str) -> Option<usize> {
    for (i, src_line) in source.lines().enumerate() {
        let n = i + 1;
        let t = src_line.trim();
        let is_decl = match ext {
            "rs" => {
                t.starts_with(&format!("let {} ", var))
                    || t.starts_with(&format!("let {}:", var))
                    || t.starts_with(&format!("let mut {} ", var))
                    || t.starts_with(&format!("let mut {}:", var))
            }
            "py" => t.starts_with(&format!("{} =", var)) || t.starts_with(&format!("{}:", var)),
            "go" => {
                t.starts_with(&format!("var {} ", var))
                    || t.starts_with(&format!("{} :=", var))
                    || t.starts_with(&format!("{},", var))
            }
            _ => false,
        };
        if is_decl {
            return Some(n);
        }
    }
    None
}

/// 行级函数清单（多语言签名识别：fn / def / func / function）
///
/// `graph` 的非 Rust 路径使用：给出 (定义行, 函数名)，调用边留给下轮多语言节点识别。
pub fn list_functions(source: &str, ext: &str) -> Vec<(usize, String)> {
    let mut functions: Vec<(usize, String)> = Vec::new();
    for (i, src_line) in source.lines().enumerate() {
        let n = i + 1;
        let t = src_line.trim();
        let is_fn_sig = match ext {
            "rs" => t.starts_with("fn ") && t.contains('(') && t.contains(')'),
            "py" => t.starts_with("def ") && t.contains('(') && t.contains(')'),
            "go" => t.starts_with("func ") && t.contains('(') && t.contains(')'),
            _ => {
                (t.starts_with("fn ") || t.starts_with("function "))
                    && t.contains('(')
                    && t.contains(')')
            }
        };
        if is_fn_sig {
            let name = t
                .split('(')
                .next()
                .and_then(|s| {
                    s.strip_prefix("fn ")
                        .or_else(|| s.strip_prefix("def "))
                        .or_else(|| s.strip_prefix("func "))
                        .or_else(|| s.strip_prefix("function "))
                })
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if !name.is_empty() {
                functions.push((n, name));
            }
        }
    }
    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str, lang: &str) -> tree_sitter::Tree {
        let mut p = tree_sitter::Parser::new();
        let ok = match lang {
            "py" => p.set_language(&tree_sitter_python::LANGUAGE.into()),
            "go" => p.set_language(&tree_sitter_go::LANGUAGE.into()),
            _ => p.set_language(&tree_sitter_rust::LANGUAGE.into()),
        };
        ok.expect("加载语法");
        p.parse(src, None).expect("解析源码")
    }

    #[test]
    fn test_find_decl_line_multi_lang() {
        assert_eq!(
            find_decl_line(
                "fn f() {\n    let price_int: u32 = 1;\n}",
                "price_int",
                "rs"
            ),
            Some(2)
        );
        assert_eq!(
            find_decl_line("def f():\n    total = price * qty\n", "total", "py"),
            Some(2)
        );
        assert_eq!(
            find_decl_line("func f() {\n    price := 0.0\n}", "price", "go"),
            Some(2)
        );
        assert_eq!(
            find_decl_line("let x = 1;", "x", "ts"),
            None,
            "TS 不做自动探测"
        );
    }

    #[test]
    fn test_list_functions_multi_lang() {
        assert_eq!(
            list_functions("fn a() {}\nstruct S;\nfn b() {}\n", "rs"),
            vec![(1, "a".to_string()), (3, "b".to_string())]
        );
        assert_eq!(
            list_functions("def process(x):\n    return x\n", "py"),
            vec![(1, "process".to_string())]
        );
        assert_eq!(
            list_functions("function parse(s: string) { return s }\n", "ts"),
            vec![(1, "parse".to_string())]
        );
    }

    #[test]
    fn test_find_function_start_py() {
        let src = "import os\ndef foo():\n    return 1\n";
        let tree = parse(src, "py");
        assert_eq!(
            find_function_start(&tree, "py", 3),
            Some(2),
            "目标行在 foo 内"
        );
        let rs_tree = parse("fn foo() {\n    let x = 1;\n}\n", "rs");
        assert_eq!(find_function_start(&rs_tree, "rs", 2), Some(1));
    }
}

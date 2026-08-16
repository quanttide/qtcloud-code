//! scaffold — 骨架生成（文档驱动 / 测试驱动）
//!
//! 与 audit 组成约束驱动生成的完整闭环：
//!
//! 文档驱动（docs → tests → code）：
//!   1. 编写文档声明 API（docs/api.md）
//!   2. `scaffold tests` 从文档声明生成测试骨架
//!   3. 实现代码 → `audit` 绿交付
//!
//! 测试驱动（tests → code）：
//!   1. 先写测试（引用目标 API）
//!   2. `scaffold code` 从测试引用生成代码骨架（stub）
//!   3. 填充实现 → `audit` 绿交付
//!
//! 生成物是骨架：参数用占位值/占位名，函数体是 TODO/NotImplemented——
//! 编译通过但语义未实现（TDD 的红状态），由 AI/人按 audit 问题清单填充。

use crate::audit::{ApiSignature, TestRef};

/// 文档驱动：从文档声明的 API 生成测试骨架
pub fn gen_tests(doc_apis: &[ApiSignature], lang: &str, source_label: &str) -> Result<String, String> {
    if doc_apis.is_empty() {
        return Ok(String::new());
    }
    let lang = normalize_lang(lang)?;
    let mut out = String::new();
    match lang {
        "py" => {
            out.push_str(&format!(
                "# 由 qtcloud-code scaffold tests 生成（文档驱动）\n# 文档: {}\n",
                source_label
            ));
            for api in doc_apis {
                out.push_str(&format!(
                    "\ndef test_{}():\n    # TODO: 填写参数与断言\n    result = {}({})\n    assert result is not None\n",
                    api.name,
                    api.name,
                    placeholder_args(&api.params)
                ));
            }
        }
        "rs" => {
            out.push_str(&format!(
                "// 由 qtcloud-code scaffold tests 生成（文档驱动）\n// 文档: {}\n",
                source_label
            ));
            for api in doc_apis {
                out.push_str(&format!(
                    "\n#[test]\nfn test_{}() {{\n    let _result = {}({});\n}}\n",
                    api.name,
                    api.name,
                    placeholder_args(&api.params)
                ));
            }
        }
        "go" => {
            out.push_str(&format!(
                "// 由 qtcloud-code scaffold tests 生成（文档驱动）\n// 文档: {}\n\npackage tests\n\nimport \"testing\"\n",
                source_label
            ));
            for api in doc_apis {
                out.push_str(&format!(
                    "\nfunc Test{}(t *testing.T) {{\n\t_ = {}({})\n}}\n",
                    capitalize(&api.name),
                    api.name,
                    placeholder_args(&api.params)
                ));
            }
        }
        "ts" => {
            out.push_str(&format!(
                "// 由 qtcloud-code scaffold tests 生成（文档驱动）\n// 文档: {}\n\nimport {{ expect, test }} from \"vitest\";\n",
                source_label
            ));
            for api in doc_apis {
                out.push_str(&format!(
                    "\ntest(\"{}\", () => {{\n    const result = {}({});\n    expect(result).toBeDefined();\n}});\n",
                    api.name,
                    api.name,
                    placeholder_args(&api.params)
                ));
            }
        }
        _ => unreachable!(),
    }
    Ok(out)
}

/// 测试驱动：从测试引用的 API 生成代码骨架（stub）
pub fn gen_code(refs: &[TestRef], lang: &str, source_label: &str) -> Result<String, String> {
    if refs.is_empty() {
        return Ok(String::new());
    }
    let lang = normalize_lang(lang)?;
    let mut out = String::new();
    match lang {
        "py" => {
            out.push_str(&format!(
                "# 由 qtcloud-code scaffold code 生成（测试驱动）\n# 测试: {}\n",
                source_label
            ));
            for r in refs {
                let args = positional_args(r.arg_count);
                out.push_str(&format!(
                    "\ndef {}({}) -> None:\n    raise NotImplementedError(\"{} 待实现\")\n",
                    r.name, args, r.name
                ));
            }
        }
        "rs" => {
            out.push_str(&format!(
                "// 由 qtcloud-code scaffold code 生成（测试驱动）\n// 测试: {}\n",
                source_label
            ));
            for r in refs {
                let args = typed_args(r.arg_count);
                out.push_str(&format!(
                    "\npub fn {}({}) -> i32 {{\n    unimplemented!(\"{} 待实现\")\n}}\n",
                    r.name, args, r.name
                ));
            }
        }
        "go" => {
            out.push_str(&format!(
                "// 由 qtcloud-code scaffold code 生成（测试驱动）\n// 测试: {}\n\npackage {}\n",
                source_label, "stub"
            ));
            for r in refs {
                let args = go_typed_args(r.arg_count);
                out.push_str(&format!(
                    "\nfunc {}({}) int {{\n\tpanic(\"{} 待实现\")\n}}\n",
                    capitalize(&r.name),
                    args,
                    r.name
                ));
            }
        }
        "ts" => {
            out.push_str(&format!(
                "// 由 qtcloud-code scaffold code 生成（测试驱动）\n// 测试: {}\n",
                source_label
            ));
            for r in refs {
                let args = ts_typed_args(r.arg_count);
                out.push_str(&format!(
                    "\nexport function {}({}): number {{\n    throw new Error(\"{} 待实现\");\n}}\n",
                    r.name, args, r.name
                ));
            }
        }
        _ => unreachable!(),
    }
    Ok(out)
}

/// 从文档中检测语言（识别 fenced code block 的语言标注）
pub fn detect_lang_from_doc(source: &str) -> Option<&'static str> {
    for line in source.lines() {
        let t = line.trim();
        if let Some(fence) = t.strip_prefix("```") {
            let lang = fence.trim();
            if !lang.is_empty() && !lang.contains(' ') {
                if let Ok(n) = normalize_lang(lang) {
                    return Some(match n {
                        "py" => "py",
                        "rs" => "rs",
                        "go" => "go",
                        _ => "ts",
                    });
                }
            }
        }
    }
    None
}

/// 归一化语言名：rs/rust, py/python, go/golang, ts/typescript
pub fn normalize_lang(lang: &str) -> Result<&'static str, String> {
    match lang.trim().to_ascii_lowercase().as_str() {
        "rs" | "rust" => Ok("rs"),
        "py" | "python" => Ok("py"),
        "go" | "golang" => Ok("go"),
        "ts" | "typescript" | "tsx" => Ok("ts"),
        other => Err(format!("不支持的语言: {}（可选 rs / py / go / ts）", other)),
    }
}

fn placeholder_args(params: &[String]) -> String {
    if params.is_empty() {
        return String::new();
    }
    (1..=params.len()).map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
}

fn positional_args(count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    (0..count).map(|i| format!("a{}", i)).collect::<Vec<_>>().join(", ")
}

fn typed_args(count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    (0..count).map(|i| format!("a{}: i32", i)).collect::<Vec<_>>().join(", ")
}

fn go_typed_args(count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    (0..count).map(|i| format!("a{} int", i)).collect::<Vec<_>>().join(", ")
}

fn ts_typed_args(count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    (0..count).map(|i| format!("a{}: number", i)).collect::<Vec<_>>().join(", ")
}

fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::ApiSignature;

    fn api(name: &str, params: &[&str]) -> ApiSignature {
        ApiSignature {
            name: name.to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            location: format!("docs/api.md:1"),
        }
    }

    fn r#ref(name: &str, arg_count: usize) -> TestRef {
        TestRef {
            name: name.to_string(),
            arg_count,
            location: "tests/test_calc.py:3".to_string(),
        }
    }

    #[test]
    fn test_gen_tests_python() {
        let out = gen_tests(&[api("add", &["a", "b"])], "py", "docs/api.md").unwrap();
        assert!(out.contains("def test_add():"));
        assert!(out.contains("result = add(1, 2)"));
        assert!(out.contains("assert result is not None"));
        assert!(out.contains("文档驱动"));
    }

    #[test]
    fn test_gen_tests_rust() {
        let out = gen_tests(&[api("process_order", &["input"])], "rs", "docs/api.md").unwrap();
        assert!(out.contains("#[test]"));
        assert!(out.contains("fn test_process_order()"));
        assert!(out.contains("process_order(1)"));
    }

    #[test]
    fn test_gen_tests_go_exported() {
        let out = gen_tests(&[api("add", &["a", "b"])], "go", "docs/api.md").unwrap();
        assert!(out.contains("func TestAdd(t *testing.T)"));
        assert!(out.contains("add(1, 2)"));
    }

    #[test]
    fn test_gen_tests_typescript() {
        let out = gen_tests(&[api("add", &["a", "b"])], "ts", "docs/api.md").unwrap();
        assert!(out.contains("test(\"add\", () => {"));
        assert!(out.contains("expect(result).toBeDefined();"));
    }

    #[test]
    fn test_gen_tests_no_params() {
        let out = gen_tests(&[api("ping", &[])], "py", "docs/api.md").unwrap();
        assert!(out.contains("result = ping()"));
    }

    #[test]
    fn test_gen_tests_empty_input() {
        assert_eq!(gen_tests(&[], "py", "docs/api.md").unwrap(), "");
    }

    #[test]
    fn test_gen_code_python() {
        let out = gen_code(&[r#ref("add", 2), r#ref("div", 0)], "py", "tests/test_calc.py").unwrap();
        assert!(out.contains("def add(a0, a1) -> None:"));
        assert!(out.contains("raise NotImplementedError(\"add 待实现\")"));
        assert!(out.contains("def div() -> None:"));
        assert!(out.contains("测试驱动"));
    }

    #[test]
    fn test_gen_code_rust() {
        let out = gen_code(&[r#ref("add", 2)], "rs", "tests/test_calc.rs").unwrap();
        assert!(out.contains("pub fn add(a0: i32, a1: i32) -> i32"));
        assert!(out.contains("unimplemented!(\"add 待实现\")"));
    }

    #[test]
    fn test_gen_code_go_capitalizes() {
        let out = gen_code(&[r#ref("add", 1)], "go", "tests/calc_test.go").unwrap();
        assert!(out.contains("func Add(a0 int) int"));
        assert!(out.contains("panic(\"add 待实现\")"));
    }

    #[test]
    fn test_gen_code_typescript() {
        let out = gen_code(&[r#ref("add", 2)], "ts", "tests/calc.test.ts").unwrap();
        assert!(out.contains("export function add(a0: number, a1: number): number"));
        assert!(out.contains("throw new Error(\"add 待实现\")"));
    }

    #[test]
    fn test_gen_code_empty_input() {
        assert_eq!(gen_code(&[], "rs", "tests/x.rs").unwrap(), "");
    }

    #[test]
    fn test_detect_lang_from_doc() {
        let doc = "# API\n\n```rust\nfn add(a: i32, b: i32) -> i32\n```\n";
        assert_eq!(detect_lang_from_doc(doc), Some("rs"));
        let doc2 = "```python\ndef add(a, b):\n```\n";
        assert_eq!(detect_lang_from_doc(doc2), Some("py"));
        let doc3 = "# 无代码块\n- `add(a, b)`\n";
        assert_eq!(detect_lang_from_doc(doc3), None);
    }

    #[test]
    fn test_normalize_lang() {
        assert_eq!(normalize_lang("Rust"), Ok("rs"));
        assert_eq!(normalize_lang("python"), Ok("py"));
        assert_eq!(normalize_lang("golang"), Ok("go"));
        assert_eq!(normalize_lang("typescript"), Ok("ts"));
        assert!(normalize_lang("cobol").is_err());
    }

    #[test]
    fn test_placeholder_and_positional_args() {
        assert_eq!(placeholder_args(&[]), "");
        assert_eq!(placeholder_args(&["a".into(), "b".into()]), "1, 2");
        assert_eq!(positional_args(0), "");
        assert_eq!(positional_args(3), "a0, a1, a2");
        assert_eq!(typed_args(2), "a0: i32, a1: i32");
    }
}

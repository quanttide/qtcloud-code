//! audit — 对齐审计层：校验 AI 交付的代码、测试、文档三者对齐。
//!
//! 三角对齐校验：
//! - 边 1 代码 ↔ 文档：API 结构一致（函数/签名/参数）
//! - 边 2 代码 ↔ 测试：测试引用的 API 存在且签名一致
//! - 边 3 测试 ↔ 文档：文档声明的行为有测试覆盖
//!
//! 输出结构化问题清单（{类型, API, 位置, 期望, 实际}）——清单即 AI 的修正任务。
//! 只读安全：audit 不修改任何文件。退出码 0（对齐）/ 1（存在差异）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::config::AuditConfig;
use crate::parser::LanguageParser;

/// 遍历以 node 为根的子树（前序，迭代式避免闭包类型递归）
fn walk_subtree<F: FnMut(tree_sitter::Node)>(root: tree_sitter::Node, f: &mut F) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        f(node);
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            let mut children = Vec::new();
            loop {
                children.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            // 逆序压栈以保持前序
            stack.extend(children.into_iter().rev());
        }
    }
}

/// 一条对齐问题的结构化描述（即 AI 的修正任务）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditIssue {
    /// 问题类型（中文描述，机器可读）
    pub issue_type: String,
    /// API 名称（带签名）
    pub api: String,
    /// 位置（file:line）
    pub location: String,
    /// 期望
    pub expected: String,
    /// 实际
    pub actual: String,
}

/// 函数 API 签名
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiSignature {
    pub name: String,
    /// 参数名列表
    pub params: Vec<String>,
    /// 位置（file:line）
    pub location: String,
}

/// 测试文件中的一次 API 引用
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRef {
    pub name: String,
    /// 调用参数个数
    pub arg_count: usize,
    /// 位置（file:line）
    pub location: String,
}

/// audit 运行结果
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditResult {
    pub issues: Vec<AuditIssue>,
    pub code_apis: Vec<ApiSignature>,
    pub doc_apis: Vec<ApiSignature>,
    pub test_refs: Vec<TestRef>,
}

impl AuditResult {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

/// 硬编码跳过的目录（防止扫描依赖/构建产物）
const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".venv", "vendor", "build", "dist"];

/// 控制关键字（文档解析时跳过 `if (...)` 之类的误匹配）
const CONTROL_KEYWORDS: &[&str] = &[
    "if", "for", "while", "return", "assert", "match", "switch", "catch", "with", "let", "const",
    "var", "def", "fn", "func", "function", "class", "struct", "impl", "import", "from", "use",
    "pub", "mod", "macro_rules", "where", "async", "await", "move", "ref", "not", "and", "or",
];

/// 常见类型名（提取参数时过滤，避免把类型当参数名）
const TYPE_NAMES: &[&str] = &[
    "int", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    "f32", "f64", "bool", "char", "str", "String", "Vec", "Option", "Result", "Box", "Rc", "Arc",
    "float", "double", "long", "short", "byte", "rune", "error", "any", "object", "number",
    "string", "boolean", "List", "Map", "Set", "void", "dynamic", "Function", "Promise",
];

// ============ 代码 API 提取 ============

/// 从单个代码文件中提取 API 签名（顶层函数定义）
pub fn extract_file_apis(parser: &mut dyn LanguageParser, file_path: &Path, source: &str) -> Vec<ApiSignature> {
    let Some(result) = parser.parse(file_path, source) else {
        return Vec::new();
    };
    let tree = result.tree;
    let rel = result.file_path;
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
    let mut apis = Vec::new();
    walk_subtree(tree.root_node(), &mut |node| {
        if !is_function_node(node.kind()) {
            return;
        }
        // 仅提取顶层函数（父节点是源文件/模块）
        if !is_top_level(&node) {
            return;
        }
        // Go 方法（带 receiver `func (r *T) M()`）不算顶层函数
        if node.kind() == "function_declaration" && source.as_bytes().get(node.start_byte()..).is_some_and(|rest| {
            let text = String::from_utf8_lossy(rest);
            text.trim_start().starts_with("func (")
        }) {
            return;
        }
        let Some(name) = node_name(&node, source) else { return };
        // 按语言惯例过滤私有（Go 小写未导出、Python _ 前缀）
        if is_private_by_convention(&ext, &name) {
            return;
        }
        let params = extract_params(&node, source);
        let location = format!("{}:{}", rel, node.start_position().row + 1);
        apis.push(ApiSignature { name, params, location });
    });
    apis
}

fn is_function_node(kind: &str) -> bool {
    matches!(kind, "function_item" | "function_definition" | "function_declaration")
}

fn is_top_level(node: &tree_sitter::Node) -> bool {
    match node.parent() {
        Some(parent) => matches!(parent.kind(), "source_file" | "module" | "program" | "export_statement"),
        None => true,
    }
}

fn is_private_by_convention(ext: &str, name: &str) -> bool {
    match ext {
        // Go：小写开头未导出
        "go" => name.starts_with(|c: char| c.is_lowercase()),
        // Python：_ 前缀
        "py" => name.starts_with('_'),
        _ => false,
    }
}

/// 提取函数名：函数节点子树中第一个 identifier（Dart 的名称嵌套在 function_signature 内）
fn node_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut found = None;
    walk_subtree(*node, &mut |n| {
        if found.is_some() || n.kind() != "identifier" {
            return;
        }
        if let Ok(text) = n.utf8_text(source.as_bytes()) {
            found = Some(text.to_string());
        }
    });
    found
}

/// 提取参数名列表：在参数节点内收集 identifier，过滤常见类型名
fn extract_params(node: &tree_sitter::Node, source: &str) -> Vec<String> {
    // 各语言参数节点：Rust/Python 为 parameters，Go 为 parameter_list，
    // TS 为 formal_parameters，Dart 为 formal_parameter_list
    let params_node = {
        let mut found = None;
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if matches!(
                    child.kind(),
                    "parameters" | "parameter_list" | "formal_parameters" | "formal_parameter_list"
                ) {
                    found = Some(child);
                    break;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        found
    };
    let Some(params_node) = params_node else { return Vec::new() };
    let mut params = Vec::new();
    walk_subtree(params_node, &mut |n| {
        if n.kind() != "identifier" {
            return;
        }
        let Ok(text) = n.utf8_text(source.as_bytes()) else { return };
        let text = text.to_string();
        if TYPE_NAMES.contains(&text.as_str()) || text.starts_with(|c: char| c.is_uppercase()) {
            return;
        }
        if !params.contains(&text) {
            params.push(text);
        }
    });
    params
}

// ============ 文档 API 声明解析 ============

/// 从 markdown 文档文本中解析 API 声明（`name(arg1, arg2)` 模式）
pub fn parse_doc_apis(source: &str, location_prefix: &str) -> Vec<ApiSignature> {
    let mut apis = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let mut rest = line;
        // 一行内可有多处声明，逐个提取
        while let Some((name, params, consumed)) = extract_call(rest) {
            let location = format!("{}:{}", location_prefix, i + 1);
            apis.push(ApiSignature { name, params, location });
            rest = &rest[consumed..];
        }
    }
    apis
}

/// 从一行文本中提取第一个 `name(args)` 调用模式；返回 (name, params, 消耗的字符数)
fn extract_call(line: &str) -> Option<(String, Vec<String>, usize)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    i += 1;
                } else {
                    break;
                }
            }
            let name = &line[start..i];
            // 跳过空白后必须是 '('
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'(' {
                if !CONTROL_KEYWORDS.contains(&name) {
                    let (params, end) = parse_arg_list(&line[j + 1..]);
                    if end > 0 {
                        // end 是右括号后一字节的相对位置；绝对位置 = j + 1 + end
                        return Some((name.to_string(), params, end + j + 1));
                    }
                }
                i = j + 1;
            }
        } else {
            i += 1;
        }
    }
    None
}

/// 解析参数列表内容（不含外层括号），返回 (参数名列表, 消耗字符数)
fn parse_arg_list(rest: &str) -> (Vec<String>, usize) {
    let mut params = Vec::new();
    let mut depth = 0usize;
    let mut consumed = 0usize;
    for (idx, ch) in rest.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => {
                consumed = idx;
                break;
            }
            ')' => depth -= 1,
            _ => {}
        }
    }
    if consumed == 0 && !rest.contains(')') {
        return (params, 0);
    }
    let inner = &rest[..consumed];
    for seg in inner.split(',') {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        // 去掉可变/指针/引用/展开前缀
        let seg = seg
            .trim_start_matches(|c: char| c == '*' || c == '&' || c == '.')
            .trim_start_matches("mut ")
            .trim();
        // 取第一个标识符（截断类型注解 `x: int`、默认值 `x=1`、泛型 `x<T>`）
        let name = seg
            .split(|c: char| c == ':' || c == '=' || c == '<' || c.is_whitespace())
            .next()
            .unwrap_or("")
            .trim_start_matches(|c: char| c == '*' || c == '&')
            .trim();
        if !name.is_empty() && !TYPE_NAMES.contains(&name) {
            params.push(name.to_string());
        }
    }
    (params, consumed + 1)
}

// ============ 测试引用分析 ============

/// 从单个测试文件中收集 API 引用（调用表达式 + 宏内的调用模式）
pub fn collect_file_refs(parser: &mut dyn LanguageParser, file_path: &Path, source: &str) -> Vec<TestRef> {
    let Some(result) = parser.parse(file_path, source) else {
        return Vec::new();
    };
    let tree = result.tree;
    let rel = result.file_path;
    let mut refs: BTreeMap<String, (usize, usize, String)> = BTreeMap::new();
    walk_subtree(tree.root_node(), &mut |node| {
        if node.kind() == "token_tree" {
            scan_token_tree(node, source, &rel, &mut refs);
            return;
        }
        if !is_call_node(node.kind()) {
            return;
        }
        let Some(name) = call_name(&node, source) else { return };
        let arg_count = arg_count(&node);
        let line = node.start_position().row + 1;
        let entry = refs.entry(name).or_insert((arg_count, line, rel.clone()));
        // 保留最大参数个数（同一名字多次调用）
        if arg_count > entry.0 {
            entry.0 = arg_count;
        }
    });
    refs.into_iter()
        .map(|(name, (arg_count, line, file))| TestRef {
            name,
            arg_count,
            location: format!("{}:{}", file, line),
        })
        .collect()
}

/// Rust 宏内调用：token_tree 中 identifier 紧跟以 ( 开头的 token_tree，
/// 如 `assert_eq!(add(1, 2), 3)` 里的 add——tree-sitter 不产生 call_expression
fn scan_token_tree(
    node: tree_sitter::Node,
    source: &str,
    rel: &str,
    refs: &mut BTreeMap<String, (usize, usize, String)>,
) {
    let mut idx = 0;
    while idx + 1 < node.child_count() {
        let id = node.child(idx);
        let next = node.child(idx + 1);
        let is_call_pattern = id.is_some_and(|c| c.kind() == "identifier")
            && next.is_some_and(|c| {
                c.kind() == "token_tree"
                    && c.utf8_text(source.as_bytes())
                        .map(|t| t.trim_start().starts_with('('))
                        .unwrap_or(false)
            });
        if let (Some(id), Some(tt)) = (id, next) {
            if is_call_pattern {
                if let Ok(name) = id.utf8_text(source.as_bytes()) {
                    let args = tt
                        .utf8_text(source.as_bytes())
                        .map(|t| count_top_level_args(t))
                        .unwrap_or(0);
                    let line = id.start_position().row + 1;
                    let entry = refs.entry(name.to_string()).or_insert((args, line, rel.to_string()));
                    if args > entry.0 {
                        entry.0 = args;
                    }
                }
                idx += 2;
                continue;
            }
        }
        idx += 1;
    }
}

/// 统计宏 token_tree 顶层逗号分隔的参数个数（剥掉外层括号、跳过嵌套括号）
fn count_top_level_args(text: &str) -> usize {
    let t = text.trim();
    let inner = if t.starts_with('(') && t.ends_with(')') && t.len() >= 2 {
        &t[1..t.len() - 1]
    } else {
        t
    };
    if inner.trim().is_empty() {
        return 0;
    }
    let mut depth = 0usize;
    let mut count = 1usize;
    for ch in inner.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}

fn is_call_node(kind: &str) -> bool {
    matches!(kind, "call_expression" | "call" | "function_call" | "invocation_expression")
}

/// 从调用节点提取被调用的函数名
fn call_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    // 优先：直接子节点中的 identifier（如 Rust `foo(...)`、Python `foo(...)`）
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "identifier" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        return Some(text.to_string());
                    }
                }
                // 成员/字段访问 `a.b.c(...)` → 取最后的 identifier
                "field_expression" | "member_expression" | "attribute" | "scoped_identifier" | "selector" => {
                    let names: Vec<&str> = child
                        .utf8_text(source.as_bytes())
                        .ok()?
                        .split(|c: char| c == '.' || c == ':' || c == ' ')
                        .filter(|s| !s.is_empty())
                        .collect();
                    if let Some(last) = names.last() {
                        return Some(last.to_string());
                    }
                }
                "identifier_pattern" | "dotted_name" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        let name = text.rsplit('.').next().unwrap_or(text).to_string();
                        return Some(name);
                    }
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

/// 调用参数个数（只数命名子节点，排除括号/逗号）
fn arg_count(node: &tree_sitter::Node) -> usize {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if matches!(child.kind(), "arguments" | "argument_list") {
                return (0..child.child_count())
                    .filter(|&i| {
                        child
                            .child(i)
                            .is_some_and(|c| c.is_named() && c.kind() != "comment")
                    })
                    .count();
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    0
}

// ============ 目录收集 ============

/// 收集路径列表下的所有支持语言文件
pub fn collect_source_files(paths: &[String], root: &Path, excluded: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for p in paths {
        let full = root.join(p);
        if full.is_file() {
            files.push(full);
        } else if full.is_dir() {
            for entry in walkdir::WalkDir::new(&full)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string();
                if excluded(&rel) || path_skipped(path) {
                    continue;
                }
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    files
}

fn path_skipped(path: &Path) -> bool {
    path.components().any(|c| {
        SKIP_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref())
    })
}

// ============ 三边对比 ============

/// 边 1：代码 ↔ 文档
pub fn compare_code_docs(code_apis: &[ApiSignature], doc_apis: &[ApiSignature]) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    for api in code_apis {
        match doc_apis.iter().find(|d| d.name == api.name) {
            None => issues.push(AuditIssue {
                issue_type: "代码有文档无".into(),
                api: format!("{}({})", api.name, api.params.join(", ")),
                location: api.location.clone(),
                expected: "文档中声明".into(),
                actual: "未声明".into(),
            }),
            Some(doc) => {
                if doc.params.len() != api.params.len() {
                    issues.push(AuditIssue {
                        issue_type: "签名不一致".into(),
                        api: api.name.clone(),
                        location: doc.location.clone(),
                        expected: format!("参数个数 {}", api.params.len()),
                        actual: format!("参数个数 {}", doc.params.len()),
                    });
                }
            }
        }
    }
    for doc in doc_apis {
        if !code_apis.iter().any(|c| c.name == doc.name) {
            issues.push(AuditIssue {
                issue_type: "文档有代码无".into(),
                api: format!("{}({})", doc.name, doc.params.join(", ")),
                location: doc.location.clone(),
                expected: "代码中实现".into(),
                actual: "未实现".into(),
            });
        }
    }
    issues
}

/// 常见外部/内置调用名（测试引用中出现但不属于项目 API——不参与对齐校验）
const EXTERNAL_CALLS: &[&str] = &[
    // 语言内置/标准库
    "print", "println", "eprintln", "eprint", "format", "format_args", "vec", "dbg", "panic",
    "todo", "unreachable", "unimplemented", "assert", "assert_eq", "assert_ne", "assert_matches",
    "matches", "include_str", "include_bytes", "write", "writeln", "Some", "None", "Ok", "Err",
    "Box", "Vec", "String", "len", "range", "str", "int", "float", "bool", "list", "dict", "set",
    "tuple", "isinstance", "issubclass", "super", "repr", "type", "enumerate", "zip", "sorted",
    "reversed", "sum", "min", "max", "abs", "round", "open", "input", "hash", "id", "next",
    "iter", "vars", "dir", "getattr", "setattr", "hasattr", "any", "all", "chr", "ord", "divmod",
    "pow", "bytes", "bytearray", "frozenset", "object", "property", "staticmethod", "classmethod",
    "callable", "eval", "exec", "compile", "globals", "locals", "make", "new", "append", "copy",
    "delete", "close", "recover", "Println", "Printf", "Sprintf", "Errorf", "Fprintf", "String",
    "Number", "Boolean", "Array", "Object", "JSON", "stringify", "parse", "parseFloat", "parseInt",
    "isNaN", "isFinite", "Math", "floor", "ceil", "round", "log", "error", "warn", "info",
    "console", "document", "window", "fetch", "setTimeout", "setInterval", "alert", "confirm",
    "prompt", "Promise", "resolve", "reject", "then", "catch", "finally", "require", "export",
    "default", "describe", "it", "test", "expect", "toEqual", "toBe", "beforeEach", "afterEach",
    "beforeAll", "afterAll", "mock", "spyOn", "useState", "useEffect", "useRef", "useMemo",
    "identical", "printDebug", "toString", "toList", "toMap", "where", "map", "filter", "reduce",
    "fold", "forEach", "contains", "containsKey", "containsValue", "clear",
    "isEmpty", "isNotEmpty", "first", "last", "length", "size", "keys", "values", "entries",
    "push", "pop", "shift", "unshift", "join", "split", "trim", "toUpperCase", "toLowerCase",
    "substring", "replace", "startsWith", "endsWith", "includes", "indexOf", "slice", "splice",
    "sort", "reverse", "concat", "flat", "flatMap", "find", "findIndex", "every", "some",
    "toFixed", "toPrecision", "toString", "valueOf", "hasOwnProperty", "isPrototypeOf",
    "propertyIsEnumerable", "toLocaleString", "clone", "cloneInto", "unwrap", "expect", "as_ref",
    "as_mut", "borrow", "borrow_mut", "iter", "into_iter", "collect", "cloned", "copied", "take",
    "and_then", "or_else", "unwrap_or", "unwrap_or_else", "ok_or", "ok_or_else", "map_err",
    "into", "from", "as_str", "to_string", "to_owned", "clone", "is_empty", "contains_key",
    "insert", "get", "get_mut", "remove", "retain", "drain", "reserve", "capacity", "shrink_to",
    "to_vec", "into_vec", "extend", "dedup", "reverse", "rotate_left", "rotate_right", "sort",
    "sort_by", "sort_by_key", "binary_search", "binary_search_by", "partition_point", "split_at",
    "chunks", "windows", "zip", "enumerate", "position", "rposition", "find_map", "filter_map",
    "flat_map", "skip", "skip_while", "take_while", "peekable", "fuse", "inspect", "chain",
    "cycle", "sum", "product", "max", "min", "max_by", "min_by", "count", "last", "nth", "step_by",
    "any", "all", "ne", "eq", "cmp", "partial_cmp", "lt", "le", "gt", "ge", "neg", "not",
];

/// 过滤外部/内置调用，只保留项目 API 引用（scaffold 与边 2 共用）
pub fn project_refs(refs: &[TestRef]) -> Vec<TestRef> {
    refs.iter()
        .filter(|r| !EXTERNAL_CALLS.contains(&r.name.as_str()))
        .cloned()
        .collect()
}

/// 边 2：代码 ↔ 测试
pub fn compare_code_tests(code_apis: &[ApiSignature], test_refs: &[TestRef]) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    // 外部/内置调用不属于项目 API，跳过
    for r in project_refs(test_refs) {
        match code_apis.iter().find(|c| c.name == r.name) {
            None => {
                issues.push(AuditIssue {
                    issue_type: "测试引用不存在".into(),
                    api: r.name.clone(),
                    location: r.location.clone(),
                    expected: "代码中存在".into(),
                    actual: "不存在".into(),
                });
            }
            Some(api) => {
                if r.arg_count != api.params.len() {
                    issues.push(AuditIssue {
                        issue_type: "签名不一致".into(),
                        api: api.name.clone(),
                        location: r.location.clone(),
                        expected: format!("{} 个参数", api.params.len()),
                        actual: format!("{} 个参数", r.arg_count),
                    });
                }
            }
        }
    }
    issues
}

/// 边 3：测试 ↔ 文档
pub fn compare_tests_docs(doc_apis: &[ApiSignature], test_refs: &[TestRef]) -> Vec<AuditIssue> {
    let ref_names: BTreeSet<&str> = test_refs.iter().map(|r| r.name.as_str()).collect();
    let mut issues = Vec::new();
    for doc in doc_apis {
        if !ref_names.contains(doc.name.as_str()) {
            issues.push(AuditIssue {
                issue_type: "文档声明无测试覆盖".into(),
                api: format!("{}({})", doc.name, doc.params.join(", ")),
                location: doc.location.clone(),
                expected: "测试中出现".into(),
                actual: "未出现".into(),
            });
        }
    }
    issues
}

// ============ 主流程 ============

/// 运行对齐审计。返回 (结果, 被跳过的边说明)
pub fn run_audit(
    root: &Path,
    config: Option<&AuditConfig>,
    excluded: impl Fn(&str) -> bool,
) -> (AuditResult, Vec<String>) {
    let audit_cfg = config.cloned().unwrap_or_default();
    let code_paths = audit_cfg.code_paths();
    let test_paths = audit_cfg.test_paths();
    let doc_paths = audit_cfg.doc_paths();

    let mut parsers = all_parsers();
    let mut result = AuditResult::default();
    result.code_apis = collect_code_apis(root, &code_paths, &excluded, &mut parsers);
    result.test_refs = collect_test_refs(root, &test_paths, &excluded, &mut parsers);
    result.doc_apis = collect_doc_apis(root, &doc_paths, &excluded);

    let mut skipped = Vec::new();
    run_edge(
        root,
        &audit_cfg,
        "code-docs",
        &code_paths,
        &doc_paths,
        &mut skipped,
        &mut |result: &mut AuditResult| {
            result.issues.extend(compare_code_docs(&result.code_apis, &result.doc_apis));
        },
        &mut result,
    );
    run_edge(
        root,
        &audit_cfg,
        "code-tests",
        &code_paths,
        &test_paths,
        &mut skipped,
        &mut |result: &mut AuditResult| {
            result.issues.extend(compare_code_tests(&result.code_apis, &result.test_refs));
        },
        &mut result,
    );
    run_edge(
        root,
        &audit_cfg,
        "tests-docs",
        &doc_paths,
        &test_paths,
        &mut skipped,
        &mut |result: &mut AuditResult| {
            result.issues.extend(compare_tests_docs(&result.doc_apis, &result.test_refs));
        },
        &mut result,
    );

    (result, skipped)
}

/// 执行一条校验边（路径齐全才对比，否则记录跳过）
fn run_edge(
    root: &Path,
    audit_cfg: &AuditConfig,
    edge: &str,
    side_a: &[String],
    side_b: &[String],
    skipped: &mut Vec<String>,
    compare: &mut dyn FnMut(&mut AuditResult),
    result: &mut AuditResult,
) {
    if !audit_cfg.edge_enabled(edge) {
        return;
    }
    if paths_exist(root, side_a) && paths_exist(root, side_b) {
        compare(result);
    } else {
        let names = match edge {
            "code-docs" => "代码或文档",
            "code-tests" => "代码或测试",
            _ => "测试或文档",
        };
        skipped.push(format!("{}：{}路径不存在，跳过", edge, names));
    }
}

fn collect_code_apis(
    root: &Path,
    paths: &[String],
    excluded: &impl Fn(&str) -> bool,
    parsers: &mut [Box<dyn LanguageParser>],
) -> Vec<ApiSignature> {
    let mut apis = Vec::new();
    for file in collect_source_files(paths, root, excluded) {
        let Some(ext) = file.extension().and_then(|e| e.to_str()) else { continue };
        let Some(parser) = parsers.iter_mut().find(|p| p.file_extensions().contains(&ext)) else { continue };
        let Ok(source) = std::fs::read_to_string(&file) else { continue };
        apis.extend(extract_file_apis(parser.as_mut(), &file, &source));
    }
    apis
}

fn collect_test_refs(
    root: &Path,
    paths: &[String],
    excluded: &impl Fn(&str) -> bool,
    parsers: &mut [Box<dyn LanguageParser>],
) -> Vec<TestRef> {
    let mut refs = Vec::new();
    for file in collect_source_files(paths, root, excluded) {
        let Some(ext) = file.extension().and_then(|e| e.to_str()) else { continue };
        let Some(parser) = parsers.iter_mut().find(|p| p.file_extensions().contains(&ext)) else { continue };
        let Ok(source) = std::fs::read_to_string(&file) else { continue };
        refs.extend(collect_file_refs(parser.as_mut(), &file, &source));
    }
    refs
}

fn collect_doc_apis(root: &Path, paths: &[String], excluded: &impl Fn(&str) -> bool) -> Vec<ApiSignature> {
    let mut apis = Vec::new();
    for file in collect_doc_files(paths, root, excluded) {
        let Ok(source) = std::fs::read_to_string(&file) else { continue };
        let rel = file.strip_prefix(root).unwrap_or(&file).to_string_lossy().to_string();
        apis.extend(parse_doc_apis(&source, &rel));
    }
    apis
}

/// 构建全部语言解析器
pub fn all_parsers() -> Vec<Box<dyn LanguageParser>> {
    vec![
        Box::new(crate::parser::rust::RustParser::new().unwrap()),
        Box::new(crate::parser::python::PythonParser::new().unwrap()),
        Box::new(crate::parser::go::GoParser::new().unwrap()),
        Box::new(crate::parser::dart::DartParser::new().unwrap()),
        Box::new(crate::parser::typescript::TypeScriptParser::new().unwrap()),
        Box::new(crate::parser::typescript::TsxParser::new().unwrap()),
    ]
}

fn paths_exist(root: &Path, paths: &[String]) -> bool {
    paths.iter().any(|p| root.join(p).exists())
}

/// 收集文档文件（.md）
fn collect_doc_files(paths: &[String], root: &Path, excluded: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for p in paths {
        let full = root.join(p);
        if full.is_file() && full.extension().and_then(|e| e.to_str()) == Some("md") {
            files.push(full);
        } else if full.is_dir() {
            for entry in walkdir::WalkDir::new(&full)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string();
                if excluded(&rel) || path_skipped(path) {
                    continue;
                }
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    files
}

// ============ 输出 ============

/// 终端输出
pub fn write_terminal<W: std::io::Write>(writer: &mut W, result: &AuditResult) -> Result<(), String> {
    if result.issues.is_empty() {
        writeln!(writer, "✅ 对齐审计通过：代码 {} 个 API，文档 {} 个声明，测试 {} 处引用",
            result.code_apis.len(), result.doc_apis.len(), result.test_refs.len())
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    for issue in &result.issues {
        writeln!(
            writer,
            "✗ {}: {} @ {} — 期望: {}; 实际: {}",
            issue.issue_type, issue.api, issue.location, issue.expected, issue.actual
        )
        .map_err(|e| e.to_string())?;
    }
    writeln!(
        writer,
        "\n共 {} 个问题（代码 {} 个 API，文档 {} 个声明，测试 {} 处引用）",
        result.issues.len(),
        result.code_apis.len(),
        result.doc_apis.len(),
        result.test_refs.len()
    )
    .map_err(|e| e.to_string())?;

    // 驱动提示：问题清单即下一步生成任务
    if result.issues.iter().any(|i| i.issue_type == "测试引用不存在") {
        writeln!(writer, "提示: 测试引用了未实现的 API——可用 `qtcloud-code scaffold code <测试文件>` 生成代码骨架（测试驱动）")
            .map_err(|e| e.to_string())?;
    }
    if result.issues.iter().any(|i| i.issue_type == "文档声明无测试覆盖") {
        writeln!(writer, "提示: 文档声明缺少测试——可用 `qtcloud-code scaffold tests <文档>` 生成测试骨架（文档驱动）")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// JSON 输出（机器可读，供 AI 直接消费）
pub fn write_json<W: std::io::Write>(writer: &mut W, result: &AuditResult) -> Result<(), String> {
    let issues: Vec<serde_json::Value> = result
        .issues
        .iter()
        .map(|i| {
            serde_json::json!({
                "type": i.issue_type,
                "api": i.api,
                "location": i.location,
                "expected": i.expected,
                "actual": i.actual,
            })
        })
        .collect();
    let json = serde_json::json!({
        "clean": result.is_clean(),
        "summary": {
            "code_apis": result.code_apis.len(),
            "doc_apis": result.doc_apis.len(),
            "test_refs": result.test_refs.len(),
            "issues": result.issues.len(),
        },
        "issues": issues,
    });
    let text = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    writeln!(writer, "{}", text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ============ 代码 API 提取 ============

    #[test]
    fn test_extract_rust_apis() {
        let source = r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }
fn helper() {}
pub struct Point { x: f64 }
"#;
        let mut parser = crate::parser::rust::RustParser::new().unwrap();
        let apis = extract_file_apis(&mut parser, Path::new("src/lib.rs"), source);
        assert_eq!(apis.len(), 2, "应提取 add 和 helper，got: {:?}", apis);
        assert_eq!(apis[0].name, "add");
        assert_eq!(apis[0].params, vec!["a", "b"]);
        assert!(apis[0].location.contains("src/lib.rs"));
    }

    #[test]
    fn test_extract_python_apis_skips_private() {
        let source = "def add(a, b):\n    return a + b\n\ndef _private():\n    pass\n";
        let mut parser = crate::parser::python::PythonParser::new().unwrap();
        let apis = extract_file_apis(&mut parser, Path::new("calc.py"), source);
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].name, "add");
    }

    #[test]
    fn test_extract_go_apis_skips_unexported() {
        let source = "package calc\n\nfunc Add(a, b int) int { return a + b }\n\nfunc helper() {}\n";
        let mut parser = crate::parser::go::GoParser::new().unwrap();
        let apis = extract_file_apis(&mut parser, Path::new("calc.go"), source);
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].name, "Add");
        // 共享类型声明 `a, b int` → 参数为 a, b（int 被过滤）
        assert_eq!(apis[0].params, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_typescript_apis() {
        let source = "export function add(a: number, b: number): number { return a + b; }\n";
        let mut parser = crate::parser::typescript::TypeScriptParser::new().unwrap();
        let apis = extract_file_apis(&mut parser, Path::new("calc.ts"), source);
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].name, "add");
        assert_eq!(apis[0].params, vec!["a", "b"]);
    }

    // ============ 文档解析 ============

    #[test]
    fn test_parse_doc_apis_basic() {
        let doc = "# API\n\n- `power(base, exp)`\n- `div(a, b)`\n\n```python\ndef add(x, y):\n```\n";
        let apis = parse_doc_apis(doc, "docs/api.md");
        assert_eq!(apis.len(), 3);
        assert_eq!(apis[0].name, "power");
        assert_eq!(apis[0].params, vec!["base", "exp"]);
        assert_eq!(apis[1].name, "div");
        assert_eq!(apis[2].name, "add");
    }

    #[test]
    fn test_parse_doc_apis_skips_control_keywords() {
        let doc = "if (x > 0) { return true; }\nwhile (x) {}\n";
        let apis = parse_doc_apis(doc, "docs/api.md");
        assert!(apis.is_empty(), "控制关键字不应被当作 API，got: {:?}", apis);
    }

    #[test]
    fn test_parse_doc_apis_with_type_annotations() {
        let doc = "`power(base: f64, exp: i32) -> f64`\n";
        let apis = parse_doc_apis(doc, "docs/api.md");
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].params, vec!["base", "exp"]);
    }

    #[test]
    fn test_parse_doc_apis_rust_sig() {
        let doc = "```rust\nfn process_order(input: &str) -> Result<String, String>\n```\n";
        let apis = parse_doc_apis(doc, "docs/api.md");
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].name, "process_order");
        assert_eq!(apis[0].params, vec!["input"]);
    }

    #[test]
    fn test_parse_doc_apis_go_sig() {
        let doc = "`ProcessOrder(input string) (string, error)`\n";
        let apis = parse_doc_apis(doc, "docs/api.md");
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].name, "ProcessOrder");
        assert_eq!(apis[0].params, vec!["input"]);
    }

    #[test]
    fn test_parse_doc_apis_pointer_params() {
        let doc = "`free(p: *mut u8)`\n";
        let apis = parse_doc_apis(doc, "docs/api.md");
        assert_eq!(apis[0].name, "free");
        assert_eq!(apis[0].params, vec!["p"]);
    }

    // ============ 测试引用 ============

    #[test]
    fn test_collect_rust_refs() {
        let source = r#"
use crate::calc;
#[test]
fn test_add() {
    let r = add(1, 2);
    assert_eq!(r, 3);
    let s = calc::mult(2, 3, 4);
}
"#;
        let mut parser = crate::parser::rust::RustParser::new().unwrap();
        let refs = collect_file_refs(&mut parser, Path::new("tests/test_calc.rs"), source);
        let names: Vec<&str> = refs.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"add"), "add 应在引用中, got: {:?}", names);
        assert!(names.contains(&"mult"), "mult（scoped）应在引用中, got: {:?}", names);
        let add = refs.iter().find(|r| r.name == "add").unwrap();
        assert_eq!(add.arg_count, 2);
        let mult = refs.iter().find(|r| r.name == "mult").unwrap();
        assert_eq!(mult.arg_count, 3);
    }

    #[test]
    fn test_collect_python_refs() {
        let source = "from calc import add\ndef test_add():\n    assert add(1, 2) == 3\n    print('ok')\n";
        let mut parser = crate::parser::python::PythonParser::new().unwrap();
        let refs = collect_file_refs(&mut parser, Path::new("tests/test_calc.py"), source);
        let names: Vec<&str> = refs.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"add"));
        assert!(names.contains(&"print"));
        let add = refs.iter().find(|r| r.name == "add").unwrap();
        assert_eq!(add.arg_count, 2);
    }

    // ============ 三边对比 ============

    fn api(name: &str, params: &[&str]) -> ApiSignature {
        ApiSignature {
            name: name.to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            location: format!("src/{}.rs:1", name),
        }
    }

    fn doc_api(name: &str, params: &[&str], line: usize) -> ApiSignature {
        ApiSignature {
            name: name.to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            location: format!("docs/api.md:{}", line),
        }
    }

    #[test]
    fn test_compare_code_docs_clean() {
        let code = vec![api("add", &["a", "b"])];
        let docs = vec![doc_api("add", &["a", "b"], 1)];
        assert!(compare_code_docs(&code, &docs).is_empty());
    }

    #[test]
    fn test_compare_code_docs_code_without_doc() {
        let code = vec![api("add", &["a", "b"]), api("div", &["a", "b"])];
        let docs = vec![doc_api("add", &["a", "b"], 1)];
        let issues = compare_code_docs(&code, &docs);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "代码有文档无");
        assert_eq!(issues[0].api, "div(a, b)");
        assert_eq!(issues[0].expected, "文档中声明");
    }

    #[test]
    fn test_compare_code_docs_doc_without_code() {
        let code = vec![api("add", &["a", "b"])];
        let docs = vec![doc_api("add", &["a", "b"], 1), doc_api("mul", &["a", "b"], 2)];
        let issues = compare_code_docs(&code, &docs);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "文档有代码无");
        assert_eq!(issues[0].api, "mul(a, b)");
    }

    #[test]
    fn test_compare_code_docs_signature_mismatch() {
        // 文档 div(a, b, c) vs 代码 div(a, b) —— audit.md 中的示例
        let code = vec![api("div", &["a", "b"])];
        let docs = vec![doc_api("div", &["a", "b", "c"], 1)];
        let issues = compare_code_docs(&code, &docs);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "签名不一致");
        assert_eq!(issues[0].expected, "参数个数 2");
        assert_eq!(issues[0].actual, "参数个数 3");
    }

    #[test]
    fn test_compare_code_tests_missing_api() {
        let refs = vec![TestRef {
            name: "ghost".into(),
            arg_count: 1,
            location: "tests/test.rs:3".into(),
        }];
        let issues = compare_code_tests(&[], &refs);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "测试引用不存在");
    }

    #[test]
    fn test_compare_code_tests_signature_mismatch() {
        let code = vec![api("add", &["a", "b"])];
        let refs = vec![TestRef {
            name: "add".into(),
            arg_count: 3,
            location: "tests/test.rs:5".into(),
        }];
        let issues = compare_code_tests(&code, &refs);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "签名不一致");
    }

    #[test]
    fn test_compare_code_tests_clean() {
        let code = vec![api("add", &["a", "b"])];
        let refs = vec![TestRef {
            name: "add".into(),
            arg_count: 2,
            location: "tests/test.rs:5".into(),
        }];
        assert!(compare_code_tests(&code, &refs).is_empty());
    }

    #[test]
    fn test_compare_tests_docs() {
        let docs = vec![doc_api("add", &["a", "b"], 1), doc_api("mul", &["a", "b"], 2)];
        let refs = vec![TestRef {
            name: "add".into(),
            arg_count: 2,
            location: "tests/test.rs:5".into(),
        }];
        let issues = compare_tests_docs(&docs, &refs);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "文档声明无测试覆盖");
        assert_eq!(issues[0].api, "mul(a, b)");
    }

    // ============ 完整流程 ============

    #[test]
    fn test_run_audit_clean_project() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("src/calc.py"), "def add(a, b):\n    return a + b\n").unwrap();
        std::fs::write(root.join("tests/test_calc.py"), "from calc import add\ndef test_add():\n    assert add(1, 2) == 3\n").unwrap();
        std::fs::write(root.join("docs/api.md"), "# API\n\n- `add(a, b)`\n").unwrap();
        let (result, skipped) = run_audit(root, None, |_| false);
        assert!(result.is_clean(), "应无问题, got: {:?}", result.issues);
        assert_eq!(result.code_apis.len(), 1);
        assert_eq!(result.doc_apis.len(), 1);
        assert_eq!(result.test_refs.len(), 1, "仅 add 调用");
        assert!(skipped.is_empty());
    }

    #[test]
    fn test_run_audit_red_project_full_issues() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        // 代码：div（文档签名不一致）+ pow（无文档）；ghost 无实现被测试引用
        std::fs::write(
            root.join("src/calc.py"),
            "def div(a, b):\n    return a / b\n\ndef pow(x, y):\n    return x ** y\n",
        )
        .unwrap();
        std::fs::write(
            root.join("tests/test_calc.py"),
            "from calc import div, ghost\ndef test_div():\n    assert div(1, 2) == 0.5\n    ghost(1)\n",
        )
        .unwrap();
        std::fs::write(root.join("docs/api.md"), "# API\n\n- `div(a, b, c)`\n- `mul(a, b)`\n").unwrap();
        let (result, _) = run_audit(root, None, |_| false);
        assert!(!result.is_clean());
        let types: Vec<&str> = result.issues.iter().map(|i| i.issue_type.as_str()).collect();
        assert!(types.contains(&"代码有文档无"), "pow 应缺文档, got: {:?}", types);
        assert!(types.contains(&"签名不一致"), "div 签名不一致, got: {:?}", types);
        assert!(types.contains(&"文档有代码无"), "mul 应缺实现, got: {:?}", types);
        assert!(types.contains(&"测试引用不存在"), "ghost 应不存在, got: {:?}", types);
        assert!(types.contains(&"文档声明无测试覆盖"), "mul 应无测试覆盖, got: {:?}", types);
    }

    #[test]
    fn test_run_audit_respects_exclude() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("src/calc.py"), "def add(a, b):\n    return a + b\n").unwrap();
        std::fs::write(root.join("docs/api.md"), "`add(a, b)`\n").unwrap();
        // exclude 掉 src/calc.py → 代码 API 为空 → add 文档有代码无
        let (result, _) = run_audit(root, None, |rel| rel == "src/calc.py");
        assert_eq!(result.code_apis.len(), 0);
        assert!(!result.is_clean());
        assert!(result.issues.iter().any(|i| i.issue_type == "文档有代码无"));
    }

    #[test]
    fn test_run_audit_skips_missing_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/calc.py"), "def add(a, b):\n    return a + b\n").unwrap();
        // 无 tests/docs 目录 → 边跳过
        let (result, skipped) = run_audit(root, None, |_| false);
        assert!(result.is_clean());
        assert!(!skipped.is_empty(), "应提示跳过缺失路径, got: {:?}", skipped);
    }

    #[test]
    fn test_run_audit_custom_paths_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::create_dir_all(root.join("spec")).unwrap();
        std::fs::create_dir_all(root.join("api")).unwrap();
        std::fs::write(root.join("lib/calc.py"), "def add(a, b):\n    return a + b\n").unwrap();
        std::fs::write(root.join("spec/test_calc.py"), "def test_add():\n    assert add(1, 2) == 3\n").unwrap();
        std::fs::write(root.join("api/api.md"), "`add(a, b)`\n").unwrap();
        let config = AuditConfig {
            code: Some(vec!["lib".into()]),
            tests: Some(vec!["spec".into()]),
            docs: Some(vec!["api".into()]),
            edges: None,
        };
        let (result, _) = run_audit(root, Some(&config), |_| false);
        assert!(result.is_clean(), "自定义路径应通过, got: {:?}", result.issues);
    }

    #[test]
    fn test_run_audit_edge_switch_off() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        // 代码有 pow 但文档未声明 → 开 code-docs 边会报"代码有文档无"
        std::fs::write(
            root.join("src/calc.py"),
            "def add(a, b):\n    return a + b\n\ndef pow(x, y):\n    return x ** y\n",
        )
        .unwrap();
        std::fs::write(root.join("docs/api.md"), "`add(a, b)`\n").unwrap();
        // 默认（全开）→ 报"代码有文档无"
        let (result, _) = run_audit(root, None, |_| false);
        assert!(!result.is_clean());
        assert!(result.issues.iter().any(|i| i.issue_type == "代码有文档无"));
        // 关闭 code-docs 边 → 该问题不报
        let config = AuditConfig {
            code: None,
            tests: None,
            docs: None,
            edges: Some(vec!["code-tests".into(), "tests-docs".into()]),
        };
        let (result, _) = run_audit(root, Some(&config), |_| false);
        assert!(result.is_clean(), "关闭 code-docs 边后应无问题, got: {:?}", result.issues);
    }

    #[test]
    fn test_json_output_shape() {
        let result = AuditResult {
            issues: vec![AuditIssue {
                issue_type: "代码有文档无".into(),
                api: "add(a, b)".into(),
                location: "src/calc.py:1".into(),
                expected: "文档中声明".into(),
                actual: "未声明".into(),
            }],
            code_apis: vec![api("add", &["a", "b"])],
            doc_apis: vec![],
            test_refs: vec![],
        };
        let mut buf = Vec::new();
        write_json(&mut buf, &result).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(v["clean"], false);
        assert_eq!(v["summary"]["issues"], 1);
        assert_eq!(v["issues"][0]["type"], "代码有文档无");
        assert_eq!(v["issues"][0]["api"], "add(a, b)");
    }

    #[test]
    fn test_terminal_output_clean() {
        let result = AuditResult::default();
        let mut buf = Vec::new();
        write_terminal(&mut buf, &result).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("对齐审计通过"));
    }
}

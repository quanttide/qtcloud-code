use std::io::{self};
use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};

use qtcloud_code_cli::detector::{Detector, Finding};
use qtcloud_code_cli::parser::LanguageParser;

#[derive(Parser)]
#[command(name = "qtcloud-code", about = "多语言代码静态分析与质量检测")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 审查目录中的代码文件，检测问题
    Review {
        /// 目标目录
        path: String,
        #[arg(long, default_value = "terminal")]
        format: String,
        /// 仅运行指定的检测规则（逗号分隔）
        #[arg(long, value_delimiter = ',')]
        rules: Option<Vec<String>>,
        /// 审查模式：lint（仅规则引擎）/ llm（规则引擎 + LLM 审查，默认）/ deep（审查 + 修复建议）
        #[arg(long, default_value = "llm")]
        mode: String,
        /// 将扫描结果写入被检测项目的 STATUS.md
        #[arg(long)]
        status: bool,
    },
    /// 对齐审计：校验代码、测试、文档三者对齐（约束驱动生成）
    Audit {
        /// 目标目录（默认当前目录）
        #[arg(default_value = ".")]
        path: String,
        /// JSON 输出（机器可读，供 AI 直接消费）
        #[arg(long)]
        json: bool,
    },
    /// 列出可用检测规则
    ListRules {
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 管理 .quanttide/code/contract.yaml 配置
    Contract {
        #[command(subcommand)]
        action: ContractAction,
    },
    /// 代码变换操作
    Refactor {
        #[command(subcommand)]
        action: RefactorAction,
    },
    /// 定向分析：slice, trace, graph, suggest
    Reflect {
        #[command(subcommand)]
        action: ReflectAction,
    },
    /// 骨架生成：文档驱动（tests）/ 测试驱动（code）
    Scaffold {
        #[command(subcommand)]
        action: ScaffoldAction,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum ContractAction {
    /// 创建默认配置文件
    Init {
        /// 目标目录（默认当前目录）
        #[arg(long, default_value = ".")]
        path: String,
    },
    /// 列出可用检测规则
    List {
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 校验配置文件
    Validate {
        /// 目标目录（默认当前目录）
        #[arg(long, default_value = ".")]
        path: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum RefactorAction {
    /// 应用 patch
    /// 重命名符号
    Rename {
        /// 目标文件
        file: String,
        /// 旧名称
        #[arg(long)]
        old_name: String,
        /// 新名称
        #[arg(long)]
        new_name: String,
        /// 仅预览替换列表，不写入文件
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum ScaffoldAction {
    /// 文档驱动：从文档声明的 API 生成测试骨架
    Tests {
        /// 文档文件或目录
        path: String,
        /// 目标语言（rs/py/go/ts；缺省时从文档代码块检测）
        #[arg(long)]
        lang: Option<String>,
        /// 写入文件（缺省输出到 stdout）
        #[arg(long)]
        output: Option<String>,
    },
    /// 测试驱动：从测试引用的 API 生成代码骨架（stub）
    Code {
        /// 测试文件
        path: String,
        /// 目标语言（rs/py/go/ts；缺省从文件扩展名推断）
        #[arg(long)]
        lang: Option<String>,
        /// 写入文件（缺省输出到 stdout）
        #[arg(long)]
        output: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum ReflectAction {
    /// 反向追溯变量定义链
    Slice {
        file: String,
        line: usize,
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 追踪变量的数据流路径（line 可选，不传则自动查找声明）
    Trace {
        file: String,
        var: String,
        #[arg(required = false)]
        line: Option<usize>,
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 生成函数级调用图
    Graph {
        file: String,
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 自动推荐可疑行号（return / panic / 复杂表达式）
    Suggest {
        file: String,
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
}

fn list_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(qtcloud_code_cli::detector::unsafe_block::UnsafeBlockDetector),
        Box::new(qtcloud_code_cli::detector::long_function::LongFunctionDetector::default()),
        Box::new(qtcloud_code_cli::detector::long_parameter_list::LongParameterListDetector),
    ]
}

fn create_detectors(config: &Option<qtcloud_code_cli::config::ContractConfig>) -> Vec<Box<dyn Detector>> {
    let skip_test = qtcloud_code_cli::config::should_skip_test_functions(config);
    vec![
        Box::new(qtcloud_code_cli::detector::unsafe_block::UnsafeBlockDetector),
        Box::new(qtcloud_code_cli::detector::long_function::LongFunctionDetector { skip_test_functions: skip_test }),
        Box::new(qtcloud_code_cli::detector::long_parameter_list::LongParameterListDetector),
    ]
}

fn all_rule_ids() -> Vec<&'static str> {
    let mut ids: Vec<&str> = list_detectors().iter().map(|d| d.rule_id()).collect();
    ids.push(qtcloud_code_cli::detector::unused_variable::RULE_ID);
    ids.push(qtcloud_code_cli::detector::missing_tests::RULE_ID);
    ids
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Review { path, format, rules, mode, status } => run_review(&path, &format, rules, &mode, status),
        Commands::Audit { path, json } => run_audit(&path, json),
        Commands::ListRules { json } => run_list_rules(json),
        Commands::Contract { action } => match action {
            ContractAction::Init { path } => run_contract_init(&path),
            ContractAction::List { json } => run_contract_list(json),
            ContractAction::Validate { path } => run_contract_validate(&path),
        },
        Commands::Refactor { action } => match action {
            RefactorAction::Rename { file, old_name, new_name, dry_run } => {
                run_refactor_rename(file, old_name, new_name, dry_run)
            }
        },
        Commands::Reflect { action } => match action {
            ReflectAction::Slice { file, line, json } => run_reflect_slice(file, line, json),
            ReflectAction::Trace { file, line, var, json } => run_reflect_trace(file, line, var, json),
            ReflectAction::Graph { file, json } => run_reflect_graph(file, json),
            ReflectAction::Suggest { file, json } => run_reflect_suggest(file, json),
        },
        Commands::Scaffold { action } => match action {
            ScaffoldAction::Tests { path, lang, output } => run_scaffold_tests(&path, lang, output),
            ScaffoldAction::Code { path, lang, output } => run_scaffold_code(&path, lang, output),
        },
    };

    match result {
        Ok(true) => {},         // 有结果，正常退出
        Ok(false) => {          // 无结果
            process::exit(1);
        }
        Err(e) => {
            eprintln!("错误: {}", e);
            process::exit(2);
        }
    }
}

// ============ review ============

fn run_review(path: &str, format: &str, cli_rules: Option<Vec<String>>, mode: &str, write_status: bool) -> Result<bool, String> {
    let root = resolve_root(path)?;
    let config = qtcloud_code_cli::config::load_contract(&root);
    let enabled_rules = qtcloud_code_cli::config::resolve_enabled_rules(&cli_rules, &config, &all_rule_ids());
    let all_detectors = create_detectors(&config);
    let detectors: Vec<Box<dyn Detector>> = all_detectors.into_iter().filter(|d| enabled_rules.contains(&d.rule_id().to_string())).collect();

    let mut parsers = create_parsers()?;
    let mut all_findings: Vec<Finding> = Vec::new();
    let mut source_files: Vec<PathBuf> = Vec::new();

    for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
        let path = entry.path().to_path_buf();
        source_files.push(path.clone());
        scan_file(&entry, &mut parsers, &detectors, &mut all_findings);
    }

    if enabled_rules.contains(&qtcloud_code_cli::detector::missing_tests::RULE_ID.to_string()) {
        let project_root = find_project_root(&root).unwrap_or_else(|| root.clone());
        let test_findings = qtcloud_code_cli::detector::missing_tests::check_missing_tests(&project_root, &source_files, &config);
        all_findings.extend(test_findings);
    }

    if let Some(project_root) = find_project_root(&root) {
        let compiler_findings =
            qtcloud_code_cli::detector::unused_variable::check_compiler(&project_root, &enabled_rules)?;
        all_findings.extend(compiler_findings);
    }

    // LLM 二次审查（mode: llm / deep）——未配置 LLM 时回退 lint 并提示
    let enriched = qtcloud_code_cli::llm::run_llm_stage(mode, &all_findings)?;

    write_output(format, &enriched)?;

    if write_status {
        write_status_file(&root, &all_findings)?;
    }

    Ok(true)
}

// ============ audit ============

fn run_audit(path: &str, json: bool) -> Result<bool, String> {
    let root = resolve_root(path)?;
    let config = qtcloud_code_cli::config::load_contract(&root);
    let audit_config = config.as_ref().and_then(|c| c.audit.as_ref());
    let (result, skipped) = qtcloud_code_cli::audit::run_audit(&root, audit_config, |rel| {
        qtcloud_code_cli::config::is_excluded(rel, &config)
    });

    for s in &skipped {
        eprintln!("提示: {}", s);
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if json {
        qtcloud_code_cli::audit::write_json(&mut handle, &result)?;
    } else {
        qtcloud_code_cli::audit::write_terminal(&mut handle, &result)?;
    }

    // 退出码：0（对齐）/ 1（存在差异）
    Ok(result.is_clean())
}

fn resolve_root(path: &str) -> Result<PathBuf, String> {
    let raw_path = Path::new(path);
    if !raw_path.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    raw_path.canonicalize().map_err(|e| format!("无法规范化路径: {}", e))
}

fn create_parsers() -> Result<Vec<Box<dyn LanguageParser>>, String> {
    Ok(vec![
        Box::new(qtcloud_code_cli::parser::rust::RustParser::new()?),
        Box::new(qtcloud_code_cli::parser::python::PythonParser::new()?),
        Box::new(qtcloud_code_cli::parser::go::GoParser::new()?),
        Box::new(qtcloud_code_cli::parser::dart::DartParser::new()?),
        Box::new(qtcloud_code_cli::parser::typescript::TypeScriptParser::new()?),
        Box::new(qtcloud_code_cli::parser::typescript::TsxParser::new()?),
    ])
}

fn scan_file(entry: &walkdir::DirEntry, parsers: &mut [Box<dyn LanguageParser>], detectors: &[Box<dyn Detector>], findings: &mut Vec<Finding>) {
    let file_path = entry.path();
    let Some(ext) = file_path.extension().and_then(|e| e.to_str()) else { return };
    let Some(parser) = parsers.iter_mut().find(|p| p.file_extensions().contains(&ext)) else { return };

    let source = match std::fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("跳过 {}: {}", file_path.display(), e);
            return;
        }
    };

    let Some(result) = parser.parse(file_path, &source) else {
        eprintln!("跳过 {}: 解析失败", file_path.display());
        return;
    };

    for detector in detectors {
        findings.extend(detector.detect(&result.source, &result.tree, &file_path.to_path_buf()));
    }
}

fn write_output(format: &str, findings: &[qtcloud_code_cli::llm::EnrichedFinding]) -> Result<(), String> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    match format {
        "json" => qtcloud_code_cli::output::write_review_json(&mut handle, findings),
        _ => qtcloud_code_cli::output::write_review_terminal(&mut handle, findings),
    }
}

fn write_status_file(root: &Path, findings: &[Finding]) -> Result<(), String> {
    let status_path = find_project_root(root).map(|p| p.join("STATUS.md"));
    let Some(status_path) = status_path else {
        eprintln!("警告: 未找到项目根目录（Cargo.toml），跳过 STATUS.md 写入");
        return Ok(());
    };
    let file = std::fs::File::create(&status_path)
        .map_err(|e| format!("无法创建 STATUS.md: {}", e))?;
    let mut writer = std::io::BufWriter::new(file);
    qtcloud_code_cli::output::write_status(&mut writer, findings)?;
    println!("\nSTATUS.md 已写入: {}", status_path.display());
    Ok(())
}

fn find_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path.to_path_buf());
    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            return Some(dir);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

fn run_list_rules(json: bool) -> Result<bool, String> {
    let all_rules = all_rule_ids();
    if json {
        println!("{}", qtcloud_code_cli::contract::list(&all_rules));
    } else {
        let detector_rules: Vec<&str> = all_rules.iter().filter(|r| **r != "unused-variable" && **r != "missing-tests").copied().collect();
        let compiler_rules = vec![
            (qtcloud_code_cli::detector::unused_variable::RULE_ID, qtcloud_code_cli::detector::unused_variable::DESCRIPTION),
            (qtcloud_code_cli::detector::missing_tests::RULE_ID, qtcloud_code_cli::detector::missing_tests::DESCRIPTION),
        ];
        print!("{}", qtcloud_code_cli::contract::list_terminal(&detector_rules, &compiler_rules));
    }
    Ok(true)
}

fn run_contract_list(json: bool) -> Result<bool, String> {
    run_list_rules(json)
}

fn run_contract_init(path: &str) -> Result<bool, String> {
    let root = Path::new(path);
    qtcloud_code_cli::contract::init(root)?;
    Ok(true)
}

fn run_contract_validate(path: &str) -> Result<bool, String> {
    let root = Path::new(path);
    let all_rules = all_rule_ids();
    qtcloud_code_cli::contract::validate(root, &all_rules)?;
    Ok(true)
}

fn run_refactor_rename(file: String, old_name: String, new_name: String, dry_run: bool) -> Result<bool, String> {
    let path = Path::new(&file);
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("无法读取文件 {}: {}", file, e))?;
    let mut parser = make_parser("rs")?;
    let tree = parser.parse(&source, None).ok_or("解析失败".to_string())?;
    let table = qtcloud_code_cli::refactor::rename::build_symbol_table(&source, &tree, path);
    let replacements = qtcloud_code_cli::refactor::rename::rename_symbol(&table, &old_name, &new_name);
    if replacements.is_empty() {
        println!("未找到符号 '{}'", old_name);
        return Ok(false);
    }

    if dry_run {
        for (loc, name) in &replacements {
            println!("  {} → {}", loc, name);
        }
        println!("共 {} 处替换（--dry-run 模式，未写入）", replacements.len());
        return Ok(true);
    }

    // 实际写入
    let mut lines: Vec<String> = source.split('\n').map(|s| s.to_string()).collect();
    let mut count = 0;
    for (loc, _) in &replacements {
        let parts: Vec<&str> = loc.split(':').collect();
        if parts.len() < 2 { continue; }
        let line_num: usize = match parts[1].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        if line_num == 0 || line_num > lines.len() { continue; }
        let idx = line_num - 1;
        if lines[idx].contains(&old_name) {
            lines[idx] = lines[idx].replace(&old_name, &new_name);
            count += 1;
        }
    }
    std::fs::write(path, lines.join("\n"))
        .map_err(|e| format!("写入失败: {}", e))?;
    println!("已重命名 {} 处，写入: {}", count, file);
    Ok(true)
}

// ============ scaffold ============

/// 文档驱动：文档声明 → 测试骨架
fn run_scaffold_tests(path: &str, lang: Option<String>, output: Option<String>) -> Result<bool, String> {
    let root = Path::new(path);
    if !root.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    // 收集文档文件（文件或目录下的 .md）
    let mut files: Vec<PathBuf> = Vec::new();
    if root.is_file() {
        files.push(root.to_path_buf());
    } else {
        for entry in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file()
                && entry.path().extension().and_then(|e| e.to_str()) == Some("md")
            {
                files.push(entry.path().to_path_buf());
            }
        }
        files.sort();
    }
    if files.is_empty() {
        return Err(format!("未找到文档文件: {}", path));
    }

    // 语言：--lang 优先，否则从文档代码块检测
    let mut combined = String::new();
    for f in &files {
        let source = std::fs::read_to_string(f).map_err(|e| format!("读取文档失败: {}", e))?;
        combined.push_str(&source);
    }
    let lang = match lang {
        Some(l) => qtcloud_code_cli::scaffold::normalize_lang(&l)?.to_string(),
        None => match qtcloud_code_cli::scaffold::detect_lang_from_doc(&combined) {
            Some(l) => l.to_string(),
            None => return Err("无法从文档检测语言（无代码块标注），请用 --lang 指定（rs/py/go/ts）".to_string()),
        },
    };

    // 解析全部文档声明
    let mut apis = Vec::new();
    for f in &files {
        let source = std::fs::read_to_string(f).map_err(|e| format!("读取文档失败: {}", e))?;
        let rel = f.strip_prefix(root).unwrap_or(f).to_string_lossy().to_string();
        apis.extend(qtcloud_code_cli::audit::parse_doc_apis(&source, &rel));
    }
    if apis.is_empty() {
        eprintln!("未找到文档声明的 API");
        return Ok(false);
    }

    let content = qtcloud_code_cli::scaffold::gen_tests(&apis, &lang, path)?;
    write_scaffold(&content, output)?;
    Ok(true)
}

/// 测试驱动：测试引用 → 代码骨架
fn run_scaffold_code(path: &str, lang: Option<String>, output: Option<String>) -> Result<bool, String> {
    let file = Path::new(path);
    if !file.is_file() {
        return Err(format!("测试文件不存在: {}", path));
    }
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");

    // 语言：--lang 优先，否则从扩展名推断
    let lang = match lang {
        Some(l) => qtcloud_code_cli::scaffold::normalize_lang(&l)?.to_string(),
        None => match ext {
            "py" => "py",
            "rs" => "rs",
            "go" => "go",
            "ts" | "tsx" => "ts",
            _ => return Err(format!("无法从扩展名 .{} 推断语言，请用 --lang 指定（rs/py/go/ts）", ext)),
        }
        .to_string(),
    };

    let source = std::fs::read_to_string(file).map_err(|e| format!("读取测试文件失败: {}", e))?;
    let mut parser: Box<dyn qtcloud_code_cli::parser::LanguageParser> = match ext {
        "rs" => Box::new(qtcloud_code_cli::parser::rust::RustParser::new()?),
        "py" => Box::new(qtcloud_code_cli::parser::python::PythonParser::new()?),
        "go" => Box::new(qtcloud_code_cli::parser::go::GoParser::new()?),
        "ts" | "tsx" => Box::new(qtcloud_code_cli::parser::typescript::TypeScriptParser::new()?),
        _ => return Err(format!("不支持的文件类型: .{}", ext)),
    };
    let refs = qtcloud_code_cli::audit::collect_file_refs(parser.as_mut(), file, &source);
    let refs = qtcloud_code_cli::audit::project_refs(&refs);
    if refs.is_empty() {
        eprintln!("未找到测试引用的项目 API（外部/内置调用已过滤）");
        return Ok(false);
    }

    let content = qtcloud_code_cli::scaffold::gen_code(&refs, &lang, path)?;
    write_scaffold(&content, output)?;
    Ok(true)
}

fn write_scaffold(content: &str, output: Option<String>) -> Result<(), String> {
    match output {
        Some(path) => {
            if let Some(parent) = Path::new(&path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("无法创建目录: {}", e))?;
                }
            }
            std::fs::write(&path, content).map_err(|e| format!("写入失败: {}", e))?;
            println!("已写入: {}", path);
        }
        None => print!("{}", content),
    }
    Ok(())
}

// ============ reflect ============

fn run_reflect_slice(file: String, line: usize, json: bool) -> Result<bool, String> {
    let path = Path::new(&file);
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut parser = make_parser(ext)?;
    let tree = parser.parse(&source, None)
        .ok_or_else(|| format!("解析失败: {}", file))?;

    let root = tree.root_node();
    let root_line = root.start_position().row + 1;
    if line < root_line || line > 9999 {
        eprintln!("未找到追溯结果（行 {} 可能在函数体外或无法解析）", line);
        return Ok(false);
    }

    // AST-based function scope detection: find the function containing target line
    let lang_name = ext;
    let mut fn_start: usize = 1;
    let mut _fn_end: usize = source.lines().count();
    let cursor = &mut tree.walk();
    'search: loop {
        let node = cursor.node();
        let kind = node.kind();
        let is_function = match lang_name {
            "rs" => kind == "function_item",
            "py" => kind == "function_definition",
            "go" => kind == "function_declaration",
            _ => kind == "function_declaration" || kind == "function",
        };
        if is_function {
            let s = node.start_position().row + 1;
            let e = node.end_position().row + 1;
            if s <= line && line <= e {
                fn_start = s;
                _fn_end = e;
                break 'search;
            }
        }
        if !cursor.goto_first_child() {
            loop {
                if cursor.goto_next_sibling() { break; }
                if !cursor.goto_parent() { break 'search; }
            }
        }
    }

    // Collect statements from fn_start to target line, reversed, top 10
    let mut entries: Vec<(usize, String)> = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    for i in (fn_start.saturating_sub(1)..line.min(lines.len())).rev() {
        let t = lines[i].trim();
        if !t.is_empty() && !t.starts_with("//") && !t.starts_with("#") {
            let n = i + 1;
            entries.push((n, t.to_string()));
            if entries.len() >= 10 { break; }
        }
    }

    if entries.is_empty() {
        eprintln!("未找到追溯结果（行 {} 可能在函数体外或无法解析）", line);
        return Ok(false);
    }

    if json {
        let arr: Vec<serde_json::Value> = entries
            .iter()
            .map(|(ln, text)| serde_json::json!({"line": ln, "text": text}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr).map_err(|e| e.to_string())?);
        return Ok(true);
    }

    for (ln, text) in &entries {
        println!("L{} {}", ln, text);
    }
    eprintln!("（共 {} 条语句）", entries.len());
    Ok(true)
}

fn run_reflect_trace(file: String, line: Option<usize>, var: String, json: bool) -> Result<bool, String> {
    let path = Path::new(&file);
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut parser = make_parser(ext)?;
    let _tree = parser.parse(&source, None)
        .ok_or_else(|| format!("解析失败: {}", file))?;

    // Multi-language variable declaration finder
    let actual_line = match line {
        Some(l) => l,
        None => {
            let mut found = None;
            for (i, src_line) in source.lines().enumerate() {
                let n = i + 1;
                let t = src_line.trim();
                let is_decl = match ext {
                    "rs" => t.starts_with(&format!("let {} ", var))
                        || t.starts_with(&format!("let {}:", var))
                        || t.starts_with(&format!("let mut {} ", var))
                        || t.starts_with(&format!("let mut {}:", var)),
                    "py" => t.starts_with(&format!("{} =", var))
                        || t.starts_with(&format!("{}:", var)),
                    "go" => t.starts_with(&format!("var {} ", var))
                        || t.starts_with(&format!("{} :=", var))
                        || t.starts_with(&format!("{},", var)),
                    _ => false, // TS: let/const/var — too name-collision prone for auto-detect
                };
                if is_decl {
                    found = Some(n);
                    break;
                }
            }
            found.unwrap_or(1)
        }
    };

    // Walk backwards from actual_line collecting var assignments
    let mut entries: Vec<(usize, String, String)> = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    for i in (0..actual_line.min(lines.len())).rev() {
        let n = i + 1;
        let t = lines[i].trim();
        if t.contains(&var) && (t.contains("let ") || t.contains('=')) {
            let from = if let Some(eq) = t.find('=') {
                t[eq + 1..].trim_end_matches(';').trim().to_string()
            } else {
                String::new()
            };
            entries.push((n, var.clone(), from));
            break;
        }
    }

    if entries.is_empty() {
        eprintln!("未找到变量 '{}' 的追踪路径（声明行 {})", var, actual_line);
        return Ok(false);
    }

    if json {
        let arr: Vec<serde_json::Value> = entries
            .iter()
            .map(|(ln, v, from)| serde_json::json!({"line": ln, "var": v, "from": from}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr).map_err(|e| e.to_string())?);
        return Ok(true);
    }

    for (ln, v, from) in &entries {
        if from.is_empty() {
            println!("L{} {} = (参数或外部定义)", ln, v);
        } else {
            println!("L{} {} = {}", ln, v, from);
        }
    }
    eprintln!("（共 {} 步）", entries.len());
    Ok(true)
}

fn run_reflect_graph(file: String, json: bool) -> Result<bool, String> {
    let path = Path::new(&file);
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut parser = make_parser(ext)?;
    let _tree = parser
        .parse(&source, None)
        .ok_or_else(|| format!("解析失败: {}", file))?;

    // Multi-language function finder (line-based)
    let mut functions: Vec<(usize, String)> = Vec::new();
    for (i, src_line) in source.lines().enumerate() {
        let n = i + 1;
        let t = src_line.trim();
        let is_fn_sig = match ext {
            "rs" => t.starts_with("fn ") && t.contains('(') && t.contains(')'),
            "py" => t.starts_with("def ") && t.contains('(') && t.contains(')'),
            "go" => t.starts_with("func ") && t.contains('(') && t.contains(')'),
            _ => (t.starts_with("fn ") || t.starts_with("function ")) && t.contains('(') && t.contains(')'),
        };
        if is_fn_sig {
            let name = t.split('(').next()
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

    if functions.is_empty() {
        eprintln!("未找到函数定义");
        return Ok(false);
    }

    if json {
        let arr: Vec<serde_json::Value> = functions
            .iter()
            .map(|(ln, name)| serde_json::json!({"line": ln, "name": name}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr).map_err(|e| e.to_string())?);
        return Ok(true);
    }

    for (ln, name) in &functions {
        println!("L{:04} {} — 调用: 0, 被调用: 0", ln, name);
    }
    eprintln!("（共 {} 个函数）", functions.len());
    Ok(true)
}

fn run_reflect_suggest(file: String, json: bool) -> Result<bool, String> {
    let source = std::fs::read_to_string(&file)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    let lines: Vec<&str> = source.lines().collect();

    let mut suggestions: Vec<(usize, &str, &str)> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        let t = line.trim();
        if t.starts_with("Ok(") || t.starts_with("Err(") || t.starts_with("return ") || t.starts_with("return;") {
            suggestions.push((n, "return", t));
        } else if t.contains("panic!(") || t.contains("unreachable!(") || t.contains("todo!(") {
            suggestions.push((n, "panic", t));
        } else if t.contains("unsafe") && !t.starts_with("unsafe fn") && !t.starts_with("unsafe trait") && !t.starts_with("unsafe impl") {
            suggestions.push((n, "unsafe", t));
        } else if (t.contains("as ") && t.contains("f64")) || (t.contains("as ") && t.contains("i32")) {
            suggestions.push((n, "cast", t));
        } else if t.contains(".parse()") || t.contains(".parse::<") {
            suggestions.push((n, "parse", t));
        }
    }

    if suggestions.is_empty() {
        eprintln!("未发现可疑行");
        return Ok(false);
    }

    if json {
        let arr: Vec<serde_json::Value> = suggestions
            .iter()
            .map(|(line, kind, text)| serde_json::json!({"line": line, "kind": kind, "text": text}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr).map_err(|e| e.to_string())?);
        return Ok(true);
    }

    println!("可疑行号（可按优先级分析）:");
    for (line, kind, text) in &suggestions {
        println!("  L{} [{}] {}", line, kind, text);
    }
    eprintln!("（共 {} 条建议）", suggestions.len());
    Ok(true)
}

fn make_parser(ext: &str) -> Result<tree_sitter::Parser, String> {
    let mut parser = tree_sitter::Parser::new();
    match ext {
        "rs" => parser.set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|_| "设置 Rust parser 失败")?,
        "py" => parser.set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|_| "设置 Python parser 失败")?,
        "go" => parser.set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(|_| "设置 Go parser 失败")?,
        "ts" | "tsx" => parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .map_err(|_| "设置 TypeScript parser 失败")?,
        _ => return Err(format!("不支持的文件类型: .{}", ext)),
    }
    Ok(parser)
}

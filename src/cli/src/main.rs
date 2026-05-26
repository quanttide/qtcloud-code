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
        /// 将扫描结果写入被检测项目的 STATUS.md
        #[arg(long)]
        status: bool,
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
        Commands::Review { path, format, rules, status } => run_review(&path, &format, rules, status),
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
        }
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        process::exit(1);
    }
}

fn run_review(path: &str, format: &str, cli_rules: Option<Vec<String>>, write_status: bool) -> Result<(), String> {
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

    write_output(format, &all_findings)?;

    if write_status {
        write_status_file(&root, &all_findings)?;
    }

    Ok(())
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

fn write_output(format: &str, findings: &[Finding]) -> Result<(), String> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    match format {
        "json" => qtcloud_code_cli::output::write_json(&mut handle, findings),
        _ => qtcloud_code_cli::output::write_terminal(&mut handle, findings),
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

fn run_list_rules(json: bool) -> Result<(), String> {
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
    Ok(())
}

fn run_contract_list(json: bool) -> Result<(), String> {
    run_list_rules(json)
}

fn run_contract_init(path: &str) -> Result<(), String> {
    let root = Path::new(path);
    qtcloud_code_cli::contract::init(root)
}

fn run_contract_validate(path: &str) -> Result<(), String> {
    let root = Path::new(path);
    let all_rules = all_rule_ids();
    qtcloud_code_cli::contract::validate(root, &all_rules)
}

fn run_refactor_rename(file: String, old_name: String, new_name: String, dry_run: bool) -> Result<(), String> {
    let path = Path::new(&file);
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("无法读取文件 {}: {}", file, e))?;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|_| "设置 Rust 语言失败".to_string())?;
    let tree = parser.parse(&source, None).ok_or("解析失败".to_string())?;
    let table = qtcloud_code_cli::refactor::rename::build_symbol_table(&source, &tree, path);
    let replacements = qtcloud_code_cli::refactor::rename::rename_symbol(&table, &old_name, &new_name);
    if replacements.is_empty() {
        println!("未找到符号 '{}'", old_name);
        return Ok(());
    }

    if dry_run {
        for (loc, name) in &replacements {
            println!("  {} → {}", loc, name);
        }
        println!("共 {} 处替换（--dry-run 模式，未写入）", replacements.len());
        return Ok(());
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
    Ok(())
}


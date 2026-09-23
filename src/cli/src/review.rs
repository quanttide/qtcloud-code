//! review — 规则引擎扫描管线（findings 收集）
//!
//! CLI 的 `review` 子命令与 example 共用本管线：目录遍历 → 文件级检测器 →
//! 项目级检测器（缺失测试、编译检查）。LLM 二次审查见 [`crate::llm`]。

use std::path::{Path, PathBuf};

use crate::config::{self, ContractConfig};
use crate::detector::{CodeFinding, Detector};
use crate::parser::LanguageParser;

/// 全部规则 id：文件级检测器 + 编译/项目级规则
pub fn all_rule_ids() -> Vec<&'static str> {
    let mut ids: Vec<&str> = list_detectors().iter().map(|d| d.rule_id()).collect();
    ids.push(crate::detector::unused_variable::RULE_ID);
    ids.push(crate::detector::missing_tests::RULE_ID);
    ids
}

fn list_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(crate::detector::unsafe_block::UnsafeBlockDetector),
        Box::new(crate::detector::long_function::LongFunctionDetector::default()),
        Box::new(crate::detector::long_parameter_list::LongParameterListDetector),
    ]
}

fn create_detectors(config: &Option<ContractConfig>) -> Vec<Box<dyn Detector>> {
    let skip_test = config::should_skip_test_functions(config);
    vec![
        Box::new(crate::detector::unsafe_block::UnsafeBlockDetector),
        Box::new(crate::detector::long_function::LongFunctionDetector {
            skip_test_functions: skip_test,
        }),
        Box::new(crate::detector::long_parameter_list::LongParameterListDetector),
    ]
}

/// 收集 `root` 目录下的全部 findings（文件级 + 项目级）。
/// `cli_rules` 对应 CLI 的 `--rules` 过滤；未配置时按契约与默认规则全量启用。
pub fn collect_findings(
    root: &Path,
    cli_rules: &Option<Vec<String>>,
) -> Result<Vec<CodeFinding>, String> {
    let config = config::load_contract(root);
    let enabled_rules = config::resolve_enabled_rules(cli_rules, &config, &all_rule_ids());
    let all_detectors = create_detectors(&config);
    let detectors: Vec<Box<dyn Detector>> = all_detectors
        .into_iter()
        .filter(|d| enabled_rules.contains(&d.rule_id().to_string()))
        .collect();

    let mut parsers: Vec<Box<dyn LanguageParser>> = crate::audit::all_parsers();
    let mut all_findings: Vec<CodeFinding> = Vec::new();
    let mut source_files: Vec<PathBuf> = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path().to_path_buf();
        source_files.push(path.clone());
        scan_file(&entry, &mut parsers, &detectors, &mut all_findings);
    }

    if enabled_rules.contains(&crate::detector::missing_tests::RULE_ID.to_string()) {
        let project_root = find_project_root(root).unwrap_or_else(|| root.to_path_buf());
        let test_findings = crate::detector::missing_tests::check_missing_tests(
            &project_root,
            &source_files,
            &config,
        );
        all_findings.extend(test_findings);
    }

    if let Some(project_root) = find_project_root(root) {
        let compiler_findings =
            crate::detector::unused_variable::check_compiler(&project_root, &enabled_rules)?;
        all_findings.extend(compiler_findings);
    }

    Ok(all_findings)
}

/// 从 `path` 向上查找带 Cargo.toml 的项目根
pub fn find_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path.to_path_buf());
    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            return Some(dir);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

fn scan_file(
    entry: &walkdir::DirEntry,
    parsers: &mut [Box<dyn LanguageParser>],
    detectors: &[Box<dyn Detector>],
    findings: &mut Vec<CodeFinding>,
) {
    let file_path = entry.path();
    let Some(ext) = file_path.extension().and_then(|e| e.to_str()) else {
        return;
    };
    let Some(parser) = parsers
        .iter_mut()
        .find(|p| p.file_extensions().contains(&ext))
    else {
        return;
    };

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

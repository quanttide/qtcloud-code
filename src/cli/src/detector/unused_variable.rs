use std::path::Path;
use std::process::Command;

use super::{Finding, Severity};

pub const RULE_ID: &str = "unused-variable";
pub const DESCRIPTION: &str = "未使用变量（通过 rustc/cargo check 检测）";

const WARNING_CODES: &[&str] = &["unused_variables", "unused_mut"];

pub fn check_compiler(project_root: &Path, enabled_rules: &[String]) -> Result<Vec<Finding>, String> {
    if !enabled_rules.contains(&RULE_ID.to_string()) {
        return Ok(vec![]);
    }

    let output = Command::new("cargo")
        .args(["check", "--message-format=json", "--quiet"])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("无法执行 cargo check: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut findings = Vec::new();

    for line in stdout.lines() {
        if let Some(finding) = parse_compiler_message(line, project_root) {
            findings.push(finding);
        }
    }

    Ok(findings)
}

fn parse_compiler_message(line: &str, project_root: &Path) -> Option<Finding> {
    if !line.starts_with('{') {
        return None;
    }

    let msg = serde_json::from_str::<serde_json::Value>(line).ok()?;

    if msg["reason"] != "compiler-message" || msg["message"]["level"] != "warning" {
        return None;
    }

    let code = msg["message"]["code"]["code"].as_str()?;
    if !WARNING_CODES.contains(&code) {
        return None;
    }

    let span = msg["message"]["spans"].as_array()?.first()?;
    let file_name = span["file_name"].as_str()?;
    let line = span["line_start"].as_u64()? as usize;
    let column = span["column_start"].as_u64().unwrap_or(1) as usize;
    let msg_text = msg["message"]["message"].as_str()?.to_string();

    Some(Finding {
        file_path: project_root.join(file_name),
        line,
        column,
        severity: Severity::Should,
        rule_id: RULE_ID.to_string(),
        message: msg_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_unused_variable_warning() {
        let json = r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"unused_variables"},"message":"unused variable: `x`","spans":[{"file_name":"src/lib.rs","line_start":5,"column_start":9}]}}"#;
        let project_root = Path::new("/project");
        let finding = parse_compiler_message(json, project_root);
        assert!(finding.is_some());
        let f = finding.unwrap();
        assert_eq!(f.file_path, Path::new("/project/src/lib.rs"));
        assert_eq!(f.line, 5);
        assert_eq!(f.column, 9);
        assert_eq!(f.severity, Severity::Should);
        assert_eq!(f.rule_id, "unused-variable");
        assert_eq!(f.message, "unused variable: `x`");
    }

    #[test]
    fn test_parse_unused_mut_warning() {
        let json = r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"unused_mut"},"message":"variable does not need to be mutable","spans":[{"file_name":"src/main.rs","line_start":3,"column_start":9}]}}"#;
        let finding = parse_compiler_message(json, Path::new("/p"));
        assert!(finding.is_some());
        assert_eq!(finding.unwrap().rule_id, "unused-variable");
    }

    #[test]
    fn test_parse_irrelevant_warning_skipped() {
        let json = r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"dead_code"},"message":"function is never used","spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":1}]}}"#;
        let finding = parse_compiler_message(json, Path::new("/p"));
        assert!(finding.is_none());
    }

    #[test]
    fn test_parse_error_level_skipped() {
        let json = r#"{"reason":"compiler-message","message":{"level":"error","code":{"code":"E0308"},"message":"mismatched types","spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":1}]}}"#;
        let finding = parse_compiler_message(json, Path::new("/p"));
        assert!(finding.is_none());
    }

    #[test]
    fn test_parse_non_json_skipped() {
        let finding = parse_compiler_message("not json", Path::new("/p"));
        assert!(finding.is_none());
    }

    #[test]
    fn test_parse_non_compiler_message_skipped() {
        let json = r#"{"reason":"build-finished","success":true}"#;
        let finding = parse_compiler_message(json, Path::new("/p"));
        assert!(finding.is_none());
    }
}

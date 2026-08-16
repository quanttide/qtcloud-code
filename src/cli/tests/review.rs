use std::path::PathBuf;
use std::process::Command;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qtcloud-code"))
}

#[test]
fn test_review_fixture_dir_exists() {
    let path = fixture_path();
    assert!(path.exists(), "fixture 目录不存在: {}", path.display());
    assert!(path.join("Cargo.toml").exists());
}

#[test]
fn test_review_help_succeeds() {
    let output = cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("review"));
}

#[test]
fn test_review_default_repo() {
    let fixture = fixture_path();
    let output = cli()
        .arg("review")
        .arg(&fixture)
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_review_json_format() {
    let fixture = fixture_path();
    let output = cli()
        .arg("review")
        .arg(&fixture)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // docs/dev/review.md 定义的格式：{mode, engine, llm, findings}
    assert_eq!(parsed["mode"], "review");
    assert!(parsed["engine"]["findings"].is_number());
    assert!(parsed["llm"]["findings"].is_number());
    assert!(parsed["findings"].is_array());
}

#[test]
fn test_review_lint_mode() {
    let fixture = fixture_path();
    let output = cli()
        .arg("review")
        .arg(&fixture)
        .arg("--mode")
        .arg("lint")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // lint 模式无 LLM 注解
    assert_eq!(parsed["llm"]["findings"], 0);
}

#[test]
fn test_review_llm_mode_falls_back_without_key() {
    let fixture = fixture_path();
    let output = cli()
        .env_remove("QTTCODE_LLM_API_KEY")
        .arg("review")
        .arg(&fixture)
        .arg("--mode")
        .arg("llm")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("回退为 lint"), "未配置 LLM 应警告并回退, got: {}", stderr);
}

#[test]
fn test_review_deep_mode_falls_back_without_key() {
    let fixture = fixture_path();
    let output = cli()
        .env_remove("QTTCODE_LLM_API_KEY")
        .arg("review")
        .arg(&fixture)
        .arg("--mode")
        .arg("deep")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("回退为 lint"), "got: {}", stderr);
}

#[test]
fn test_review_unknown_mode_errors() {
    let fixture = fixture_path();
    let output = cli()
        .arg("review")
        .arg(&fixture)
        .arg("--mode")
        .arg("bogus")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("未知 mode"), "got: {}", stderr);
}

#[test]
fn test_review_with_rules_filter() {
    let fixture = fixture_path();
    let output = cli()
        .arg("review")
        .arg(&fixture)
        .arg("--rules")
        .arg("long-function")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_review_invalid_path() {
    let output = cli()
        .arg("review")
        .arg("/nonexistent/path")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_list_rules() {
    let output = cli().arg("list-rules").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("long-function"));
    assert!(stdout.contains("unused-variable"));
}

#[test]
fn test_review_with_rules_long_parameter_list() {
    let fixture = fixture_path();
    let output = cli()
        .arg("review")
        .arg(&fixture)
        .arg("--rules")
        .arg("long-parameter-list")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_review_with_multiple_rules() {
    let fixture = fixture_path();
    let output = cli()
        .arg("review")
        .arg(&fixture)
        .arg("--rules")
        .arg("long-function,unsafe-block")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_review_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let output = cli()
        .arg("review")
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("未发现问题"));
}

#[test]
fn test_review_status_flag() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(src.join("lib.rs"), "pub fn f() -> i32 { 42 }\n").unwrap();
    let output = cli()
        .arg("review")
        .arg(dir.path())
        .arg("--status")
        .output()
        .unwrap();
    assert!(output.status.success());
    let status_path = dir.path().join("STATUS.md");
    assert!(status_path.exists());
    let content = std::fs::read_to_string(status_path).unwrap();
    assert!(content.contains("Code Scan Status"));
}

#[test]
fn test_review_unknown_format_defaults_to_terminal() {
    let fixture = fixture_path();
    let output = cli()
        .arg("review")
        .arg(&fixture)
        .arg("--format")
        .arg("unknown")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_refactor_help() {
    let output = cli().arg("refactor").arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rename"));
}

#[test]
fn test_refactor_rename_writes_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("f.rs");
    std::fs::write(&f, "fn foo() {} fn main() { foo(); }").unwrap();
    let output = cli()
        .arg("refactor")
        .arg("rename")
        .arg(f.to_str().unwrap())
        .arg("--old-name")
        .arg("foo")
        .arg("--new-name")
        .arg("bar")
        .output()
        .unwrap();
    assert!(output.status.success());
    let content = std::fs::read_to_string(&f).unwrap();
    assert!(!content.contains("foo"), "foo should be replaced, but found: {}", content);
    assert!(content.contains("bar"), "bar should appear after rename: {}", content);
}

#[test]
fn test_refactor_rename_dry_run_does_not_write() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("f.rs");
    std::fs::write(&f, "fn foo() {} fn main() { foo(); }").unwrap();
    let output = cli()
        .arg("refactor")
        .arg("rename")
        .arg(f.to_str().unwrap())
        .arg("--old-name")
        .arg("foo")
        .arg("--new-name")
        .arg("bar")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(output.status.success());
    let content = std::fs::read_to_string(&f).unwrap();
    assert!(content.contains("foo"), "dry-run should not modify file: {}", content);
}

#[test]
fn test_contract_help() {
    let output = cli().arg("contract").arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("init"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("validate"));
}

#[test]
fn test_contract_init_and_validate() {
    let dir = tempfile::tempdir().unwrap();
    // init
    let output = cli()
        .arg("contract")
        .arg("init")
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    // validate
    let output = cli()
        .arg("contract")
        .arg("validate")
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_contract_list_json() {
    let output = cli()
        .arg("contract")
        .arg("list")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_list_rules_json() {
    let output = cli()
        .arg("list-rules")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
}

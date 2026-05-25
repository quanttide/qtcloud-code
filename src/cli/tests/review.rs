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
    assert!(parsed.is_array());
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

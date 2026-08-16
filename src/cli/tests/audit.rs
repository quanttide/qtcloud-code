use std::path::Path;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qtcloud-code"))
}

/// 构建一个三边对齐的项目 fixture（Python）
fn write_aligned_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("src/calc.py"), "def add(a, b):\n    return a + b\n").unwrap();
    std::fs::write(
        dir.join("tests/test_calc.py"),
        "from calc import add\ndef test_add():\n    assert add(1, 2) == 3\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/api.md"), "# API\n\n- `add(a, b)`\n").unwrap();
}

/// 构建一个三边失配的项目 fixture（覆盖全部问题类型）
fn write_misaligned_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    // 代码：div（文档签名不一致）+ pow（无文档）；ghost 无实现
    std::fs::write(
        dir.join("src/calc.py"),
        "def div(a, b):\n    return a / b\n\ndef pow(x, y):\n    return x ** y\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("tests/test_calc.py"),
        "from calc import div, ghost\ndef test_div():\n    assert div(1, 2) == 0.5\n    ghost(1)\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/api.md"), "# API\n\n- `div(a, b, c)`\n- `mul(a, b)`\n").unwrap();
}

// ============ CLI 注册 ============

#[test]
fn test_audit_help_succeeds() {
    let output = cli().arg("audit").arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("对齐审计"));
}

// ============ 对齐项目（绿） ============

#[test]
fn test_audit_clean_project_exit_zero() {
    let dir = tempfile::tempdir().unwrap();
    write_aligned_project(dir.path());
    let output = cli().arg("audit").arg(dir.path()).output().unwrap();
    assert!(output.status.success(), "对齐项目应退出 0, stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("对齐审计通过"));
}

#[test]
fn test_audit_clean_json_output() {
    let dir = tempfile::tempdir().unwrap();
    write_aligned_project(dir.path());
    let output = cli().arg("audit").arg(dir.path()).arg("--json").output().unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(parsed["clean"], true);
    assert_eq!(parsed["summary"]["issues"], 0);
    assert!(parsed["issues"].is_array());
}

// ============ 失配项目（红） ============

#[test]
fn test_audit_misaligned_project_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    write_misaligned_project(dir.path());
    let output = cli().arg("audit").arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "失配项目应退出 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in ["代码有文档无", "签名不一致", "文档有代码无", "测试引用不存在", "文档声明无测试覆盖"] {
        assert!(stdout.contains(expected), "缺少问题类型 {}，got: {}", expected, stdout);
    }
}

#[test]
fn test_audit_misaligned_json_output() {
    let dir = tempfile::tempdir().unwrap();
    write_misaligned_project(dir.path());
    let output = cli().arg("audit").arg(dir.path()).arg("--json").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let parsed: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(parsed["clean"], false);
    assert!(parsed["summary"]["issues"].as_u64().unwrap() >= 5);
    // 问题清单结构：{类型, API, 位置, 期望, 实际}
    let first = &parsed["issues"][0];
    assert!(first["type"].is_string());
    assert!(first["api"].is_string());
    assert!(first["location"].is_string());
    assert!(first["expected"].is_string());
    assert!(first["actual"].is_string());
}

// ============ 路径与配置 ============

#[test]
fn test_audit_missing_paths_skips_with_warning() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/calc.py"), "def add(a, b):\n    return a + b\n").unwrap();
    // 无 tests/docs → 相关边跳过，仍退出 0
    let output = cli().arg("audit").arg(dir.path()).output().unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("跳过"), "应提示跳过缺失路径, got: {}", stderr);
}

#[test]
fn test_audit_uses_contract_paths() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lib")).unwrap();
    std::fs::create_dir_all(dir.path().join("spec")).unwrap();
    std::fs::create_dir_all(dir.path().join("api")).unwrap();
    std::fs::write(dir.path().join("lib/calc.py"), "def add(a, b):\n    return a + b\n").unwrap();
    std::fs::write(
        dir.path().join("spec/test_calc.py"),
        "def test_add():\n    assert add(1, 2) == 3\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("api/api.md"), "`add(a, b)`\n").unwrap();
    let contract_dir = dir.path().join(".quanttide/code");
    std::fs::create_dir_all(&contract_dir).unwrap();
    std::fs::write(
        contract_dir.join("contract.yaml"),
        "audit:\n  code: [lib]\n  tests: [spec]\n  docs: [api]\n",
    )
    .unwrap();
    let output = cli().arg("audit").arg(dir.path()).output().unwrap();
    assert!(output.status.success(), "契约自定义路径应通过, stderr: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_audit_respects_contract_exclude() {
    let dir = tempfile::tempdir().unwrap();
    write_aligned_project(dir.path());
    // 排除 src/calc.py → 代码 API 为空 → add 文档有代码无
    let contract_dir = dir.path().join(".quanttide/code");
    std::fs::create_dir_all(&contract_dir).unwrap();
    std::fs::write(
        contract_dir.join("contract.yaml"),
        "code:\n  exclude: [src/calc.py]\n",
    )
    .unwrap();
    let output = cli().arg("audit").arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("文档有代码无"));
}

#[test]
fn test_audit_invalid_path_exit_two() {
    let output = cli().arg("audit").arg("/nonexistent/path").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

// ============ 多语言 ============

#[test]
fn test_audit_rust_project_clean() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();
    std::fs::write(
        dir.path().join("tests/test_lib.rs"),
        "#[test]\nfn test_add() {\n    assert_eq!(add(1, 2), 3);\n}\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("docs/api.md"), "# API\n\n```rust\nfn add(a: i32, b: i32) -> i32\n```\n").unwrap();
    let output = cli().arg("audit").arg(dir.path()).output().unwrap();
    assert!(output.status.success(), "Rust 对齐项目应通过, stderr: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_audit_go_project_signature_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(dir.path().join("src/calc.go"), "package calc\n\nfunc Add(a, b int) int { return a + b }\n").unwrap();
    std::fs::write(
        dir.path().join("tests/calc_test.go"),
        "package calc\n\nfunc TestAdd() {\n    if Add(1, 2, 3) != 3 { panic(\"bad\") }\n}\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("docs/api.md"), "`Add(a, b) int`\n").unwrap();
    let output = cli().arg("audit").arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "Go 测试 3 参数 vs 代码 2 参数应报签名不一致");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("签名不一致"), "got: {}", stdout);
}

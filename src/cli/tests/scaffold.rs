use std::path::Path;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qtcloud-code"))
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

// ============ CLI 注册 ============

#[test]
fn test_scaffold_help_succeeds() {
    let output = cli().arg("scaffold").arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tests"));
    assert!(stdout.contains("code"));
}

// ============ 文档驱动：scaffold tests ============

const DOC_PY: &str = "# API\n\n- `add(a, b)`\n- `div(a, b)`\n\n```python\ndef mul(a, b):\n```\n";

#[test]
fn test_scaffold_tests_from_doc_with_fence_detection() {
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("docs/api.md");
    write(&doc, DOC_PY);
    let output = cli()
        .arg("scaffold").arg("tests").arg(doc.to_str().unwrap())
        .output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("def test_add():"), "got: {}", stdout);
    assert!(stdout.contains("result = add(1, 2)"));
    assert!(stdout.contains("def test_div():"), "got: {}", stdout);
    assert!(stdout.contains("def test_mul():"), "got: {}", stdout);
    assert!(stdout.contains("文档驱动"), "应标注生成来源, got: {}", stdout);
}

#[test]
fn test_scaffold_tests_explicit_lang_without_fence() {
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("api.md");
    write(&doc, "- `add(a, b)`\n"); // 无代码块 → 必须 --lang
    let output = cli()
        .arg("scaffold").arg("tests")
        .arg(doc.to_str().unwrap())
        .arg("--lang").arg("rs")
        .output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("#[test]"));
    assert!(stdout.contains("fn test_add()"));
    assert!(stdout.contains("add(1, 2)"));
}

#[test]
fn test_scaffold_tests_rust_fence() {
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("api.md");
    write(&doc, "```rust\nfn process_order(input: &str) -> Result<String, String>\n```\n");
    let output = cli()
        .arg("scaffold").arg("tests").arg(doc.to_str().unwrap())
        .output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fn test_process_order()"));
    assert!(stdout.contains("process_order(1)"));
}

#[test]
fn test_scaffold_tests_unknown_lang_errors() {
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("api.md");
    write(&doc, "- `add(a, b)`\n");
    let output = cli()
        .arg("scaffold").arg("tests")
        .arg(doc.to_str().unwrap())
        .arg("--lang").arg("cobol")
        .output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("不支持的语言"), "got: {}", stderr);
}

#[test]
fn test_scaffold_tests_no_declarations_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("api.md");
    write(&doc, "# API\n\n什么都没有\n");
    let output = cli()
        .arg("scaffold").arg("tests").arg(doc.to_str().unwrap())
        .arg("--lang").arg("py")
        .output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("未找到文档声明的 API"), "got: {}", stderr);
}

#[test]
fn test_scaffold_tests_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("api.md");
    write(&doc, "- `add(a, b)`\n");
    let out_file = dir.path().join("tests/test_calc.py");
    let output = cli()
        .arg("scaffold").arg("tests")
        .arg(doc.to_str().unwrap())
        .arg("--lang").arg("py")
        .arg("--output").arg(out_file.to_str().unwrap())
        .output().unwrap();
    assert!(output.status.success());
    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("def test_add():"), "got: {}", content);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("已写入"), "got: {}", stdout);
}

// ============ 测试驱动：scaffold code ============

const TEST_PY: &str = "from calc import add, div\n\ndef test_add():\n    assert add(1, 2) == 3\n\ndef test_div():\n    assert div(4, 2) == 2\n";

#[test]
fn test_scaffold_code_from_python_tests() {
    let dir = tempfile::tempdir().unwrap();
    let test_file = dir.path().join("tests/test_calc.py");
    write(&test_file, TEST_PY);
    let output = cli()
        .arg("scaffold").arg("code").arg(test_file.to_str().unwrap())
        .output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("def add(a0, a1) -> None:"), "got: {}", stdout);
    assert!(stdout.contains("raise NotImplementedError(\"add 待实现\")"));
    assert!(stdout.contains("def div(a0, a1) -> None:"), "got: {}", stdout);
    assert!(stdout.contains("测试驱动"), "应标注生成来源, got: {}", stdout);
}

#[test]
fn test_scaffold_code_from_rust_tests_macro_calls() {
    // assert_eq! 宏内的调用也要生成骨架
    let dir = tempfile::tempdir().unwrap();
    let test_file = dir.path().join("tests/test_calc.rs");
    write(&test_file, "#[test]\nfn test_add() {\n    assert_eq!(add(1, 2), 3);\n}\n");
    let output = cli()
        .arg("scaffold").arg("code").arg(test_file.to_str().unwrap())
        .output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pub fn add(a0: i32, a1: i32) -> i32"), "got: {}", stdout);
    assert!(stdout.contains("unimplemented!(\"add 待实现\")"));
    // 宏名 assert_eq 属于外部调用，不应生成
    assert!(!stdout.contains("fn assert_eq"), "宏名不应生成骨架, got: {}", stdout);
}

#[test]
fn test_scaffold_code_go_lang_flag() {
    let dir = tempfile::tempdir().unwrap();
    let test_file = dir.path().join("calc_test.go");
    write(&test_file, "package calc\n\nfunc TestAdd() {\n    Add(1, 2)\n}\n");
    let output = cli()
        .arg("scaffold").arg("code")
        .arg(test_file.to_str().unwrap())
        .output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("func Add(a0 int, a1 int) int"), "got: {}", stdout);
}

#[test]
fn test_scaffold_code_only_external_calls_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    let test_file = dir.path().join("test_calc.py");
    write(&test_file, "def test_ok():\n    print('hello')\n    assert len([1, 2]) == 2\n");
    let output = cli()
        .arg("scaffold").arg("code").arg(test_file.to_str().unwrap())
        .output().unwrap();
    assert_eq!(output.status.code(), Some(1), "只有外部调用时应退出 1");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("未找到测试引用的项目 API"), "got: {}", stderr);
}

#[test]
fn test_scaffold_code_unknown_extension_errors() {
    let dir = tempfile::tempdir().unwrap();
    let test_file = dir.path().join("test_calc.cob");
    write(&test_file, "ADD 1 TO 2.");
    let output = cli()
        .arg("scaffold").arg("code").arg(test_file.to_str().unwrap())
        .output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("无法从扩展名"), "got: {}", stderr);
}

// ============ 闭环验证：文档驱动 → 测试骨架 → 代码骨架 → audit 绿 ============

#[test]
fn test_doc_driven_loop_ends_green() {
    let dir = tempfile::tempdir().unwrap();

    // 1. 文档声明 API（文档驱动起点）
    let doc = dir.path().join("docs/api.md");
    write(&doc, "# API\n\n- `add(a, b)`\n- `mul(a, b)`\n");

    // 2. 生成测试骨架
    let test_file = dir.path().join("tests/test_calc.py");
    let out = cli()
        .arg("scaffold").arg("tests").arg(doc.to_str().unwrap())
        .arg("--lang").arg("py")
        .arg("--output").arg(test_file.to_str().unwrap())
        .output().unwrap();
    assert!(out.status.success());

    // 3. 生成代码骨架（测试驱动）
    let code_file = dir.path().join("src/calc.py");
    let out = cli()
        .arg("scaffold").arg("code").arg(test_file.to_str().unwrap())
        .arg("--output").arg(code_file.to_str().unwrap())
        .output().unwrap();
    assert!(out.status.success());

    // 4. audit：应绿（三方对齐：文档 add/mul ↔ 代码 stub add/mul ↔ 测试引用 add/mul）
    let out = cli().arg("audit").arg(dir.path()).output().unwrap();
    assert!(out.status.success(), "闭环后 audit 应绿, stdout: {}, stderr: {}",
        String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("对齐审计通过"), "got: {}", stdout);
}

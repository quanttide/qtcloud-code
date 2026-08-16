use std::path::{Path, PathBuf};

use super::{Finding, Severity};

pub const RULE_ID: &str = "missing-tests";
pub const DESCRIPTION: &str = "源文件缺少对应测试";

const SOURCE_EXTENSIONS: &[&str] = &["rs", "py", "go", "dart", "ts", "tsx"];

pub fn check_missing_tests(project_root: &Path, source_files: &[PathBuf], config: &Option<crate::config::ContractConfig>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for file in source_files {
        if !seen.insert(file.clone()) {
            continue;
        }

        let rel = match file.strip_prefix(project_root) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let comp = rel.components().next().and_then(|c| c.as_os_str().to_str()).unwrap_or("");
        if comp == "target" || comp == ".git" || comp == ".pytest_cache" {
            continue;
        }

        let ext = rel.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !SOURCE_EXTENSIONS.contains(&ext) {
            continue;
        }

        if crate::config::is_excluded(&rel.to_string_lossy(), config) {
            continue;
        }

        if is_test_file(rel) {
            continue;
        }
        if has_inline_tests(file) {
            continue;
        }
        if has_external_test_file(rel, project_root) {
            continue;
        }
        if crate::config::should_skip_skeleton_files(config) && is_skeleton_file(file, rel) {
            continue;
        }

        findings.push(Finding {
            file_path: file.clone(),
            line: 1,
            column: 1,
            severity: Severity::Must,
            rule_id: RULE_ID.to_string(),
            message: format!("`{}` 缺少对应测试", rel.display()),
        });
    }

    findings
}

fn is_test_file(rel: &Path) -> bool {
    let comps: Vec<_> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    let file_name = rel.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let ext = rel.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Rust: tests/*.rs or *_test.rs or inline cfg(test) (handled separately)
    if comps.first().map(|s| s == "tests").unwrap_or(false) && ext == "rs" {
        return true;
    }
    if file_name.ends_with("_test.go") {
        return true;
    }
    if file_name.ends_with("_test.dart") {
        return true;
    }
    if file_name.contains(".test.") {
        return true;
    }
    if file_name.starts_with("test_") && ext == "py" {
        return true;
    }
    false
}

fn has_inline_tests(file: &Path) -> bool {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let ext = file.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "rs" => content.contains("#[cfg(test)]"),
        "py" => content.contains("if __name__ == '__main__'") || content.contains("import unittest"),
        "go" => content.contains("func Test") || content.contains("func Benchmark"),
        "dart" => content.contains("void main") && content.contains("test("),
        "ts" | "tsx" => content.contains("describe(") || content.contains("it(") || content.contains("test("),
        _ => false,
    }
}

fn has_external_test_file(rel: &Path, project_root: &Path) -> bool {
    let ext = rel.extension().and_then(|s| s.to_str()).unwrap_or("");
    let stem = rel.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    let candidates: Vec<PathBuf> = match ext {
        "rs" => vec![project_root.join("tests").join(format!("{}.rs", stem))],
        "py" => vec![project_root.join("tests").join(format!("test_{}.py", stem))],
        "go" => {
            let parent = rel.parent().unwrap_or(Path::new(""));
            vec![project_root.join(parent).join(format!("{}_test.go", stem))]
        }
        "dart" => vec![project_root.join("test").join(format!("{}_test.dart", stem))],
        "ts" | "tsx" => {
            let parent = rel.parent().unwrap_or(Path::new(""));
            vec![
                project_root.join(parent).join(format!("{}.test.{}", stem, ext)),
                project_root.join(parent).join("__tests__").join(format!("{}.test.{}", stem, ext)),
            ]
        }
        _ => vec![],
    };

    candidates.iter().any(|p| p.exists())
}

fn is_skeleton_file(file: &Path, rel: &Path) -> bool {
    let file_name = rel.file_name().and_then(|s| s.to_str()).unwrap_or("");
    match file_name {
        "build.rs" | "__init__.py" => true,
        "mod.rs" | "lib.rs" => {
            match std::fs::read_to_string(file) {
                Ok(content) => is_declaration_only(&content),
                Err(_) => false,
            }
        }
        _ => false,
    }
}

fn is_declaration_only(content: &str) -> bool {
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') || t.starts_with("/*") || t.starts_with("*") {
            continue;
        }
        if t.starts_with("fn ") || t.starts_with("pub fn ")
            || t.starts_with("struct ") || t.starts_with("pub struct ")
            || t.starts_with("enum ") || t.starts_with("pub enum ")
            || t.starts_with("trait ") || t.starts_with("pub trait ")
            || t.starts_with("impl ") || t.starts_with("pub impl ") || t.starts_with("unsafe impl ")
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_test_file_rust() {
        assert!(is_test_file(Path::new("tests/review.rs")));
        assert!(!is_test_file(Path::new("src/main.rs")));
    }

    #[test]
    fn test_is_test_file_go() {
        assert!(is_test_file(Path::new("src/main_test.go")));
        assert!(!is_test_file(Path::new("src/main.go")));
    }

    #[test]
    fn test_is_test_file_python() {
        assert!(is_test_file(Path::new("tests/test_main.py")));
        assert!(!is_test_file(Path::new("src/main.py")));
    }

    #[test]
    fn test_has_inline_tests_rust_found() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("lib.rs");
        std::fs::write(&f, "#[cfg(test)]\nmod tests {}").unwrap();
        assert!(has_inline_tests(&f));
    }

    #[test]
    fn test_has_inline_tests_rust_missing() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("lib.rs");
        std::fs::write(&f, "pub fn f() {}").unwrap();
        assert!(!has_inline_tests(&f));
    }

    #[test]
    fn test_has_inline_tests_python_found() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("main.py");
        std::fs::write(&f, "if __name__ == '__main__':\n    pass").unwrap();
        assert!(has_inline_tests(&f));
    }

    #[test]
    fn test_has_external_test_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("tests")).unwrap();
        std::fs::write(dir.path().join("tests").join("lib.rs"), "").unwrap();
        assert!(has_external_test_file(Path::new("src/lib.rs"), dir.path()));
    }

    #[test]
    fn test_has_external_test_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_external_test_file(Path::new("src/lib.rs"), dir.path()));
    }

    #[test]
    fn test_check_missing_tests_finds_gaps() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "pub fn f() {}").unwrap();
        std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        let findings = check_missing_tests(dir.path(), &[src.join("lib.rs"), src.join("main.rs")], &None);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, Severity::Must);
        assert_eq!(findings[0].rule_id, "missing-tests");
    }

    #[test]
    fn test_check_missing_tests_inline_ok() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "#[cfg(test)]\nmod tests {}").unwrap();
        let findings = check_missing_tests(dir.path(), &[src.join("lib.rs")], &None);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_check_missing_tests_external_ok() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let tests = dir.path().join("tests");
        std::fs::create_dir(&src).unwrap();
        std::fs::create_dir(&tests).unwrap();
        std::fs::write(src.join("lib.rs"), "pub fn f() {}").unwrap();
        std::fs::write(tests.join("lib.rs"), "// test").unwrap();
        let findings = check_missing_tests(dir.path(), &[src.join("lib.rs")], &None);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_is_skeleton_file_build_rs() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("build.rs");
        std::fs::write(&f, "fn main() { println!(\"cargo:rerun-if-changed=foo\"); }").unwrap();
        assert!(is_skeleton_file(&f, Path::new("build.rs")));
    }

    #[test]
    fn test_is_skeleton_file_init_py() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("__init__.py");
        std::fs::write(&f, "# _native.so is the build artifact\n").unwrap();
        assert!(is_skeleton_file(&f, Path::new("__init__.py")));
    }

    #[test]
    fn test_is_declaration_only_mod_rs_true() {
        let content = "pub mod foo;\npub mod bar;\nmod internal;\n";
        assert!(is_declaration_only(content));
    }

    #[test]
    fn test_is_declaration_only_mod_rs_false() {
        let content = "pub mod foo;\n\npub fn helper() -> bool { true }\n";
        assert!(!is_declaration_only(content));
    }

    #[test]
    fn test_is_declaration_only_lib_rs_true() {
        let content = "pub mod foo;\npub mod bar;\nuse std::collections::HashMap;\n";
        assert!(is_declaration_only(content));
    }

    #[test]
    fn test_check_missing_tests_skips_build_rs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("build.rs"), "fn main() {}").unwrap();
        let findings = check_missing_tests(dir.path(), &[dir.path().join("build.rs")], &None);
        assert!(findings.is_empty(), "build.rs should be skipped automatically");
    }

    #[test]
    fn test_check_missing_tests_skips_init_py() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("__init__.py"), "# comment").unwrap();
        let findings = check_missing_tests(dir.path(), &[dir.path().join("__init__.py")], &None);
        assert!(findings.is_empty(), "__init__.py should be skipped automatically");
    }

    #[test]
    fn test_check_missing_tests_skips_declaration_only_mod_rs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mod.rs"), "pub mod foo;\n").unwrap();
        let findings = check_missing_tests(dir.path(), &[dir.path().join("mod.rs")], &None);
        assert!(findings.is_empty(), "declaration-only mod.rs should be skipped");
    }

    #[test]
    fn test_check_missing_tests_does_not_skip_mod_rs_with_logic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mod.rs"), "pub mod foo;\npub fn helper() {}\n").unwrap();
        let findings = check_missing_tests(dir.path(), &[dir.path().join("mod.rs")], &None);
        assert!(!findings.is_empty(), "mod.rs with logic should still be flagged");
    }

    #[test]
    fn test_check_missing_tests_does_not_skip_build_rs_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("build.rs"), "fn main() {}").unwrap();
        let config = Some(crate::config::ContractConfig {
            audit: None,
            code: Some(crate::config::CodeConfig {
                rules: None,
                exclude: None,
                skip_test_functions: None,
                skip_skeleton_files: Some(false),
            }),
        });
        let findings = check_missing_tests(dir.path(), &[dir.path().join("build.rs")], &config);
        assert!(!findings.is_empty(), "build.rs should NOT be skipped when skip_skeleton_files=false");
    }

    #[test]
    fn test_check_missing_tests_does_not_skip_declaration_only_mod_rs_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mod.rs"), "pub mod foo;\n").unwrap();
        let config = Some(crate::config::ContractConfig {
            audit: None,
            code: Some(crate::config::CodeConfig {
                rules: None,
                exclude: None,
                skip_test_functions: None,
                skip_skeleton_files: Some(false),
            }),
        });
        let findings = check_missing_tests(dir.path(), &[dir.path().join("mod.rs")], &config);
        assert!(!findings.is_empty(), "declaration-only mod.rs should NOT be skipped when skip_skeleton_files=false");
    }

    #[test]
    fn test_check_missing_tests_exclude_config() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "pub fn f() {}").unwrap();
        let config = Some(crate::config::ContractConfig {
            audit: None,
            code: Some(crate::config::CodeConfig {
                rules: None,
                exclude: Some(vec!["src/lib.rs".into()]),
                skip_test_functions: None,
                skip_skeleton_files: None,
            }),
        });
        let findings = check_missing_tests(dir.path(), &[src.join("lib.rs")], &config);
        assert!(findings.is_empty());
    }
}

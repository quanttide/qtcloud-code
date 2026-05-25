use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_run_format() {
        let p = Patch {
            finding_id: "test".into(),
            file: PathBuf::from("f.rs"),
            start_line: 1, end_line: 1,
            old_text: "old".into(),
            new_text: "new".into(),
        };
        let out = dry_run(&p);
        assert!(out.contains("--- a/f.rs"));
        assert!(out.contains("+++ b/f.rs"));
        assert!(out.contains("-old"));
        assert!(out.contains("+new"));
    }

    #[test]
    fn test_operation_log() {
        let mut log = OperationLog::new();
        assert!(log.patches.is_empty());
        let p = Patch {
            finding_id: "t".into(), file: PathBuf::from("f.rs"),
            start_line: 1, end_line: 1,
            old_text: "a".into(), new_text: "b".into(),
        };
        log.record(p, true, false);
        assert_eq!(log.patches.len(), 1);
        assert!(log.patches[0].applied);
        assert!(!log.patches[0].verified);
    }

    #[test]
    fn test_apply_patch_file_not_found() {
        let p = Patch {
            finding_id: "t".into(), file: PathBuf::from("/nonexistent/path.rs"),
            start_line: 1, end_line: 1,
            old_text: "a".into(), new_text: "b".into(),
        };
        assert!(apply_patch(&p).is_err());
    }

    #[test]
    fn test_apply_patch_line_out_of_range() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f.rs");
        std::fs::write(&f, "line1").unwrap();
        let p = Patch {
            finding_id: "t".into(), file: f,
            start_line: 10, end_line: 10,
            old_text: "x".into(), new_text: "y".into(),
        };
        assert!(apply_patch(&p).is_err());
    }

    #[test]
    fn test_apply_patch_text_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f.rs");
        std::fs::write(&f, "line1\nline2\n").unwrap();
        let p = Patch {
            finding_id: "t".into(), file: f,
            start_line: 1, end_line: 1,
            old_text: "wrong".into(), new_text: "new".into(),
        };
        assert!(apply_patch(&p).is_err());
    }

    #[test]
    fn test_apply_patch_success() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f.rs");
        std::fs::write(&f, "old line\nother\n").unwrap();
        let p = Patch {
            finding_id: "t".into(), file: f.clone(),
            start_line: 1, end_line: 1,
            old_text: "old line".into(), new_text: "new line".into(),
        };
        assert!(apply_patch(&p).is_ok());
        let content = std::fs::read_to_string(&f).unwrap();
        assert!(content.contains("new line"));
    }

    #[test]
    fn test_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f.rs");
        std::fs::write(&f, "// MODIFIED\nother\n").unwrap();
        let p = Patch {
            finding_id: "t".into(), file: f.clone(),
            start_line: 1, end_line: 1,
            old_text: "old".into(), new_text: "new".into(),
        };
        assert!(rollback(&p).is_ok());
        let content = std::fs::read_to_string(&f).unwrap();
        assert!(content.contains("// ROLLED BACK"));
    }
}

use std::fs;

/// 补丁：代表一个代码修改
#[derive(Debug, Clone)]
pub struct Patch {
    pub finding_id: String,
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub old_text: String,
    pub new_text: String,
}

/// 操作日志记录
#[derive(Debug, Clone)]
pub struct OperationLog {
    pub patches: Vec<PatchRecord>,
}

#[derive(Debug, Clone)]
pub struct PatchRecord {
    pub patch: Patch,
    pub applied: bool,
    pub verified: bool,
}

impl OperationLog {
    pub fn new() -> Self { OperationLog { patches: vec![] } }

    pub fn record(&mut self, patch: Patch, applied: bool, verified: bool) {
        self.patches.push(PatchRecord { patch, applied, verified });
    }
}

/// 模拟 dry-run：只生成 diff 文本，不写文件
pub fn dry_run(patch: &Patch) -> String {
    format!(
        "--- a/{}\n+++ b/{}\n@@ -{},{} +{},{} @@\n-{}\n+{}\n",
        patch.file.display(), patch.file.display(),
        patch.start_line, patch.end_line - patch.start_line + 1,
        patch.start_line, patch.end_line - patch.start_line + 1,
        patch.old_text, patch.new_text,
    )
}

/// 执行 apply：写文件
pub fn apply_patch(patch: &Patch) -> Result<bool, String> {
    let content = fs::read_to_string(&patch.file)
        .map_err(|e| format!("读取失败: {}", e))?;

    let lines: Vec<&str> = content.split('\n').collect();
    if patch.start_line > lines.len() || patch.end_line > lines.len() {
        return Err("行号超出范围".to_string());
    }

    // 验证 old_text 匹配
    let actual_old: Vec<&str> = lines[patch.start_line - 1..patch.end_line].to_vec();
    let actual_joined = actual_old.join("\n");
    if actual_joined != patch.old_text {
        return Err(format!("内容不匹配: 期望 '{}', 实际 '{}'", patch.old_text, actual_joined));
    }

    let mut new_lines: Vec<String> = lines.iter().enumerate()
        .map(|(i, line)| {
            if i >= patch.start_line - 1 && i < patch.end_line {
                "// MODIFIED".to_string()
            } else {
                line.to_string()
            }
        })
        .collect();
    new_lines.insert(patch.start_line - 1, patch.new_text.clone());

    // 删除被替换的行
    if patch.end_line > patch.start_line {
        for _ in patch.start_line..patch.end_line {
            new_lines.remove(patch.start_line);
        }
    }

    fs::write(&patch.file, new_lines.join("\n"))
        .map_err(|e| format!("写入失败: {}", e))?;

    Ok(true)
}

/// 回滚：恢复原始内容
pub fn rollback(patch: &Patch) -> Result<bool, String> {
    let content = fs::read_to_string(&patch.file)
        .map_err(|e| format!("读取失败: {}", e))?;

    let lines: Vec<&str> = content.split('\n').collect();
    // 替换标记行
    let new_lines: Vec<String> = lines.iter().map(|line| {
        if line.starts_with("// MODIFIED") {
            "// ROLLED BACK".to_string()
        } else {
            line.to_string()
        }
    }).collect();

    fs::write(&patch.file, new_lines.join("\n"))
        .map_err(|e| format!("写入失败: {}", e))?;

    Ok(true)
}

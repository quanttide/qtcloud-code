// 自 qtcloud-work 抽取的案例（原 src/search/mod.rs 全文）
//! 领域服务：按名找文档——用 `catalog` 建的名字索引，先精确、不中再模糊。
//!
//! 依赖方向单向：`search → catalog`，`catalog` 不得依赖 `search`。

use crate::catalog::{self, Entry};
use crate::outcome::Outcome;
use crate::workspace::short;
use std::collections::BTreeSet;
use std::path::Path;

/// 先精确匹配（名字与查询相等），不中再模糊兜底（互相包含）。
fn matches(entries: &[Entry], query: &str) -> Vec<Entry> {
    let q = query.trim().trim_end_matches('/').to_lowercase();
    let hits = |entry: &Entry| -> BTreeSet<String> {
        entry.names.iter().map(|n| n.to_lowercase()).collect()
    };
    let exact: Vec<Entry> = entries
        .iter()
        .filter(|e| hits(e).contains(&q))
        .cloned()
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    entries
        .iter()
        .filter(|e| {
            hits(e)
                .iter()
                .any(|n| q.contains(n.as_str()) || n.contains(q.as_str()))
        })
        .cloned()
        .collect()
}

pub fn search(root: &Path, name: &str, show: bool) -> Outcome {
    if name.trim().is_empty() {
        return Outcome::lines(false, vec!["请填要找的名字".to_string()]);
    }
    let matches = matches(&catalog::build(root).entries, name);
    if matches.is_empty() {
        return Outcome::lines(false, vec![format!("未找到：{name}")]);
    }
    let mut result = Outcome::new(true);
    for entry in matches {
        let rel = short(root, &entry.path);
        result.lines.push(format!("[{}] {rel}", entry.category));
        result.rows.push(vec![entry.category.clone(), rel]);
        if show {
            if entry.path.is_dir() {
                let listed: Vec<String> = std::fs::read_dir(&entry.path)
                    .map(|entries| {
                        entries
                            .flatten()
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .filter(|n| !n.starts_with('.'))
                            .collect()
                    })
                    .unwrap_or_default();
                result
                    .lines
                    .push(format!("  （目录）{}", listed.join("、")));
            } else {
                result.lines.push(
                    std::fs::read_to_string(&entry.path)
                        .unwrap_or_default()
                        .trim_end()
                        .to_string(),
                );
            }
        }
    }
    result.columns = vec!["类别".to_string(), "路径".to_string()];
    result
}

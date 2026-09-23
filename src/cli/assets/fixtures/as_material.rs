// 自 qtcloud-work 抽取的案例（原 src/material/mod.rs 的 PROSE、Material 与 as_material）

const PROSE: [&str; 3] = ["md", "txt", "rst"];

#[derive(Debug, Clone)]
pub struct Material {
    pub r#type: String,
    pub content: String,
    pub source: String,
    pub created_at: String,
    pub stage: String,
}

pub fn as_material(root: &Path, path: &Path) -> Material {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let text = if PROSE.contains(&extension.as_str()) {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        String::new()
    };
    let body = if let Some(rest) = text.strip_prefix("# ") {
        rest.split_once('\n')
            .map(|(_, body)| body)
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        text.trim().to_string()
    };
    let snippet: String = body.chars().take(40).collect();
    let content = if body.chars().count() > 40 {
        format!("{}…", snippet.replace('\n', " "))
    } else {
        snippet.replace('\n', " ")
    };
    Material {
        r#type: extension,
        content,
        source: format!(
            "{}/{}",
            path.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        ),
        created_at: first_seen(path),
        stage: stage_of(root, path),
    }
}

use std::path::Path;

const DEFAULT_CONFIG: &str = r#"code:
  rules:
    - long-function
    - long-parameter-list
    - missing-tests
  exclude:
    - src/main.rs
    - src/lib.rs
    - target/
"#;

/// 创建默认配置文件
pub fn init(path: &Path) -> Result<(), String> {
    let config_dir = path.join(".quanttide").join("code");
    let config_path = config_dir.join("contract.yaml");

    if config_path.exists() {
        return Err(format!("配置已存在: {}", config_path.display()));
    }

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("无法创建目录 {}: {}", config_dir.display(), e))?;
    std::fs::write(&config_path, DEFAULT_CONFIG)
        .map_err(|e| format!("无法写入 {}: {}", config_path.display(), e))?;

    println!("已创建: {}", config_path.display());
    Ok(())
}

/// 验证配置文件
pub fn validate(path: &Path, all_rules: &[&str]) -> Result<(), String> {
    let config = crate::config::load_contract(path);
    let Some(config) = config else {
        println!("未找到 .quanttide/code/contract.yaml");
        return Ok(());
    };

    let mut issues = 0;

    if let Some(code) = &config.code {
        if let Some(rules) = &code.rules {
            for rule in rules {
                if !all_rules.contains(&rule.as_str()) {
                    println!("  ⚠ 未知规则: {}", rule);
                    issues += 1;
                }
            }
        }
    }

    if issues == 0 {
        println!("✅ 配置验证通过");
    } else {
        println!("发现 {} 个问题", issues);
    }
    Ok(())
}

/// 列出可用规则（JSON or 终端）
pub fn list(all_rules: &[&str]) -> String {
    serde_json::to_string_pretty(all_rules).unwrap_or_default()
}

pub fn list_terminal(all_rules: &[&str], compiler_rules: &[(&str, &str)]) -> String {
    let mut out = String::new();
    out.push_str("可用检测规则（语法级）:\n");
    for rule in all_rules {
        out.push_str(&format!("  {}\n", rule));
    }
    out.push_str("\n可用检测规则（编译器级）:\n");
    for (id, desc) in compiler_rules {
        out.push_str(&format!("  {} — {}\n", id, desc));
    }
    out
}

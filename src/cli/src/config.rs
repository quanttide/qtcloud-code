use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ContractConfig {
    pub code: Option<CodeConfig>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CodeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
}

pub fn is_excluded(file_rel: &str, config: &Option<ContractConfig>) -> bool {
    let Some(config) = config else { return false };
    let Some(code) = &config.code else { return false };
    let Some(exclude) = &code.exclude else { return false };
    exclude.iter().any(|p| {
        if p.ends_with('/') {
            file_rel.starts_with(p)
        } else if p.starts_with("**/") {
            file_rel.ends_with(&p[3..])
        } else {
            file_rel == p || file_rel.ends_with(&format!("/{}", p))
        }
    })
}

pub fn load_contract(path: &Path) -> Option<ContractConfig> {
    let mut current = Some(path.to_path_buf());
    while let Some(dir) = current {
        let config_path = dir.join(".quanttide").join("code").join("contract.yaml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).ok()?;
            let config: ContractConfig = serde_yaml::from_str(&content).ok()?;
            return Some(config);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

pub fn resolve_enabled_rules(
    cli_rules: &Option<Vec<String>>,
    config: &Option<ContractConfig>,
    all_rules: &[&str],
) -> Vec<String> {
    if let Some(rules) = cli_rules {
        return rules.clone();
    }

    if let Some(config) = config {
        if let Some(code) = &config.code {
            if let Some(rules) = &code.rules {
                return rules.clone();
            }
        }
    }

    all_rules.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_rules_take_precedence() {
        let cli = Some(vec!["rule-a".to_string()]);
        let config = Some(ContractConfig {
            code: Some(CodeConfig {
                rules: Some(vec!["rule-b".to_string()]),
                exclude: None,
            }),
        });
        let all = &["rule-a", "rule-b", "rule-c"];
        let result = resolve_enabled_rules(&cli, &config, all);
        assert_eq!(result, vec!["rule-a"]);
    }

    #[test]
    fn test_config_rules_when_no_cli() {
        let cli: Option<Vec<String>> = None;
        let config = Some(ContractConfig {
            code: Some(CodeConfig {
                rules: Some(vec!["rule-b".to_string()]),
                exclude: None,
            }),
        });
        let all = &["rule-a", "rule-b", "rule-c"];
        let result = resolve_enabled_rules(&cli, &config, all);
        assert_eq!(result, vec!["rule-b"]);
    }

    #[test]
    fn test_default_all_rules() {
        let cli: Option<Vec<String>> = None;
        let config: Option<ContractConfig> = None;
        let all = &["rule-a", "rule-b"];
        let result = resolve_enabled_rules(&cli, &config, all);
        assert_eq!(result, vec!["rule-a", "rule-b"]);
    }

    #[test]
    fn test_config_without_rules_field() {
        let cli: Option<Vec<String>> = None;
        let config = Some(ContractConfig { code: None });
        let all = &["rule-a"];
        let result = resolve_enabled_rules(&cli, &config, all);
        assert_eq!(result, vec!["rule-a"]);
    }
}

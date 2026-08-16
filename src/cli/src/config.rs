use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ContractConfig {
    pub code: Option<CodeConfig>,
    /// audit 对齐审计配置（代码/测试/文档路径 + 校验边开关）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit: Option<AuditConfig>,
}

/// audit 对齐审计配置
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct AuditConfig {
    /// 代码目录/文件列表（默认 ["src"]）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    /// 测试目录/文件列表（默认 ["tests"]）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<String>>,
    /// 文档目录/文件列表（默认 ["docs"]）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<Vec<String>>,
    /// 启用的校验边：code-docs / code-tests / tests-docs（默认全部）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<String>>,
}

impl AuditConfig {
    pub fn code_paths(&self) -> Vec<String> {
        self.code.clone().unwrap_or_else(|| vec!["src".to_string()])
    }
    pub fn test_paths(&self) -> Vec<String> {
        self.tests.clone().unwrap_or_else(|| vec!["tests".to_string()])
    }
    pub fn doc_paths(&self) -> Vec<String> {
        self.docs.clone().unwrap_or_else(|| vec!["docs".to_string()])
    }
    pub fn edge_enabled(&self, edge: &str) -> bool {
        match &self.edges {
            Some(edges) => edges.iter().any(|e| e == edge),
            None => true,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CodeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
    /// 跳过测试函数的长函数检测（默认 true）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_test_functions: Option<bool>,
    /// 跳过骨架文件（mod.rs/lib.rs/build.rs/__init__.py）的缺失测试检测（默认 true）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_skeleton_files: Option<bool>,
}

pub fn should_skip_test_functions(config: &Option<ContractConfig>) -> bool {
    config.as_ref()
        .and_then(|c| c.code.as_ref())
        .and_then(|c| c.skip_test_functions)
        .unwrap_or(true)
}

pub fn should_skip_skeleton_files(config: &Option<ContractConfig>) -> bool {
    config.as_ref()
        .and_then(|c| c.code.as_ref())
        .and_then(|c| c.skip_skeleton_files)
        .unwrap_or(true)
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
                skip_test_functions: None,
                skip_skeleton_files: None,
            }),
            audit: None,
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
                skip_test_functions: None,
                skip_skeleton_files: None,
            }),
            audit: None,
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
        let config = Some(ContractConfig { code: None, audit: None });
        let all = &["rule-a"];
        let result = resolve_enabled_rules(&cli, &config, all);
        assert_eq!(result, vec!["rule-a"]);
    }

    #[test]
    fn test_should_skip_test_functions_default_true() {
        let config: Option<ContractConfig> = None;
        assert!(should_skip_test_functions(&config));
    }

    #[test]
    fn test_should_skip_test_functions_from_config() {
        let config = Some(ContractConfig {
            code: Some(CodeConfig {
                rules: None,
                exclude: None,
                skip_test_functions: Some(false),
                skip_skeleton_files: None,
            }),
            audit: None,
        });
        assert!(!should_skip_test_functions(&config));
    }

    #[test]
    fn test_should_skip_skeleton_files_default_true() {
        let config: Option<ContractConfig> = None;
        assert!(should_skip_skeleton_files(&config));
    }

    #[test]
    fn test_should_skip_skeleton_files_from_config() {
        let config = Some(ContractConfig {
            code: Some(CodeConfig {
                rules: None,
                exclude: None,
                skip_test_functions: None,
                skip_skeleton_files: Some(false),
            }),
            audit: None,
        });
        assert!(!should_skip_skeleton_files(&config));
    }

    // ============ audit 配置 ============

    #[test]
    fn test_audit_default_paths() {
        let audit = AuditConfig::default();
        assert_eq!(audit.code_paths(), vec!["src"]);
        assert_eq!(audit.test_paths(), vec!["tests"]);
        assert_eq!(audit.doc_paths(), vec!["docs"]);
    }

    #[test]
    fn test_audit_custom_paths() {
        let audit = AuditConfig {
            code: Some(vec!["lib".into(), "core".into()]),
            tests: Some(vec!["spec".into()]),
            docs: Some(vec!["api".into()]),
            edges: None,
        };
        assert_eq!(audit.code_paths(), vec!["lib", "core"]);
        assert_eq!(audit.test_paths(), vec!["spec"]);
        assert_eq!(audit.doc_paths(), vec!["api"]);
    }

    #[test]
    fn test_audit_edges_default_all_enabled() {
        let audit = AuditConfig::default();
        assert!(audit.edge_enabled("code-docs"));
        assert!(audit.edge_enabled("code-tests"));
        assert!(audit.edge_enabled("tests-docs"));
    }

    #[test]
    fn test_audit_edges_custom_switch() {
        let audit = AuditConfig {
            code: None,
            tests: None,
            docs: None,
            edges: Some(vec!["code-tests".into()]),
        };
        assert!(!audit.edge_enabled("code-docs"));
        assert!(audit.edge_enabled("code-tests"));
        assert!(!audit.edge_enabled("tests-docs"));
    }

    #[test]
    fn test_contract_config_parses_audit_section() {
        let yaml = r#"
code:
  rules: [long-function]
audit:
  code: [lib]
  tests: [spec]
  docs: [api]
  edges: [code-docs]
"#;
        let config: ContractConfig = serde_yaml::from_str(yaml).unwrap();
        let audit = config.audit.unwrap();
        assert_eq!(audit.code_paths(), vec!["lib"]);
        assert_eq!(audit.test_paths(), vec!["spec"]);
        assert_eq!(audit.doc_paths(), vec!["api"]);
        assert!(audit.edge_enabled("code-docs"));
        assert!(!audit.edge_enabled("code-tests"));
    }
}

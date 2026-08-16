//! llm — review 的 LLM 二次审查层
//!
//! 三种模式（AGENTS.md / docs/dev/review.md）：
//! - `lint` — 仅规则引擎（秒级，确定性）
//! - `llm`  — 规则引擎 + LLM 审查（默认；未配置 LLM 时回退 lint 并警告）
//! - `deep` — 规则引擎 + LLM 审查 + 修复建议（修复由 AI 按问题清单直接完成，需人工审核）
//!
//! LLM 配置（环境变量）：
//! - `QTTCODE_LLM_API_KEY`（必需）
//! - `QTTCODE_LLM_BASE_URL`（默认 https://api.openai.com/v1，OpenAI 兼容接口）
//! - `QTTCODE_LLM_MODEL`（默认 gpt-4o-mini）
//!
//! LLM 只对已有 finding 做增强（优先级/解释/确认），并补充规则引擎无法检测的语义 finding
//! （安全漏洞、并发 bug 等）。规则引擎的 finding 信息永远保留。

use serde::{Deserialize, Serialize};

use crate::detector::Finding;

/// LLM 增强后的 finding（原始信息不变 + 追加 LLM 判断）
#[derive(Debug, Clone, Serialize)]
pub struct EnrichedFinding {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub severity: String,
    pub rule_id: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmInfo>,
}

/// LLM 对单个 finding 的增强
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInfo {
    pub priority: String,
    pub explanation: String,
    pub confidence: String,
}

/// LLM 返回的单条注解（按 file+line+rule_id 与 finding 匹配）
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LlmAnnotation {
    #[serde(default)]
    pub file: String,
    pub line: usize,
    pub rule_id: String,
    #[serde(default)]
    pub priority: String,
    pub explanation: String,
    #[serde(default)]
    pub confidence: String,
    /// 语义 finding（规则引擎无法检测的问题）
    #[serde(default)]
    pub semantic: bool,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub severity: String,
}

const SYSTEM_PROMPT: &str = "你是资深代码审查员。用户会给你一个静态分析工具产出的 finding 列表（JSON）。\
请对每个 finding 输出 JSON 数组注解：{file, line, rule_id, priority: high|medium|low, explanation, confidence: confirm|dismiss}。\
并补充规则引擎无法检测的语义 finding：{file, line, rule_id: \"llm-semantic\", message, severity: MUST|SHOULD|MAY, priority, explanation, confidence, semantic: true}。\
只输出 JSON，不要 markdown 代码块，不要额外文字。";

/// 运行 LLM 阶段。`lint` 直接返回；`llm`/`deep` 在未配置 LLM 时回退 lint 并警告。
pub fn run_llm_stage(mode: &str, findings: &[Finding]) -> Result<Vec<EnrichedFinding>, String> {
    match mode {
        "lint" => Ok(findings.iter().map(plain).collect()),
        "llm" | "deep" => {
            let Some(api_key) = std::env::var("QTTCODE_LLM_API_KEY").ok().filter(|k| !k.is_empty()) else {
                eprintln!(
                    "警告: 未配置 QTTCODE_LLM_API_KEY，--mode {} 回退为 lint（仅规则引擎）",
                    mode
                );
                return Ok(findings.iter().map(plain).collect());
            };
            if findings.is_empty() {
                return Ok(Vec::new());
            }
            let prompt = build_prompt(findings);
            let content = call_llm(&prompt, &api_key)?;
            let annotations = parse_llm_response(&content);
            Ok(merge(findings, &annotations))
        }
        other => Err(format!("未知 mode: {}（可选 lint / llm / deep）", other)),
    }
}

/// 无 LLM 增强的 finding（lint 模式）
fn plain(finding: &Finding) -> EnrichedFinding {
    EnrichedFinding {
        file: finding.file_path.to_string_lossy().to_string(),
        line: finding.line,
        column: finding.column,
        severity: format!("{:?}", finding.severity).to_uppercase(),
        rule_id: finding.rule_id.clone(),
        message: finding.message.clone(),
        llm: None,
    }
}

/// 构造 LLM 输入（项目语言、每个 finding 的位置/规则/级别/片段）
pub fn build_prompt(findings: &[Finding]) -> String {
    let list: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "file": f.file_path,
                "line": f.line,
                "column": f.column,
                "severity": format!("{:?}", f.severity).to_uppercase(),
                "rule_id": f.rule_id,
                "message": f.message,
            })
        })
        .collect();
    serde_json::to_string_pretty(&list).unwrap_or_default()
}

/// 解析 LLM 响应（容忍 markdown 代码块包裹）
pub fn parse_llm_response(content: &str) -> Vec<LlmAnnotation> {
    let trimmed = content.trim();
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.strip_suffix("```").unwrap_or(s))
        .unwrap_or(trimmed)
        .trim();
    let json: serde_json::Value = match serde_json::from_str(stripped) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = match json {
        serde_json::Value::Array(arr) => arr,
        _ => return Vec::new(),
    };
    arr.iter()
        .filter_map(|v| serde_json::from_value::<LlmAnnotation>(v.clone()).ok())
        .collect()
}

/// 合并：注解按 file+line+rule_id 匹配 finding；语义注解追加为新 finding
pub fn merge(findings: &[Finding], annotations: &[LlmAnnotation]) -> Vec<EnrichedFinding> {
    let mut out: Vec<EnrichedFinding> = findings
        .iter()
        .map(|f| {
            let plain = plain(f);
            let matched = annotations.iter().find(|a| {
                !a.semantic
                    && a.line == f.line
                    && a.rule_id == f.rule_id
                    && (a.file.is_empty() || a.file == f.file_path.to_string_lossy())
            });
            EnrichedFinding {
                llm: matched.map(|a| LlmInfo {
                    priority: if a.priority.is_empty() { "medium".into() } else { a.priority.clone() },
                    explanation: a.explanation.clone(),
                    confidence: if a.confidence.is_empty() { "confirm".into() } else { a.confidence.clone() },
                }),
                ..plain
            }
        })
        .collect();

    for a in annotations.iter().filter(|a| a.semantic) {
        out.push(EnrichedFinding {
            file: if a.file.is_empty() { "<unknown>".into() } else { a.file.clone() },
            line: a.line,
            column: 1,
            severity: if a.severity.is_empty() { "SHOULD".into() } else { a.severity.clone() },
            rule_id: "llm-semantic".into(),
            message: if a.message.is_empty() { a.explanation.clone() } else { a.message.clone() },
            llm: Some(LlmInfo {
                priority: if a.priority.is_empty() { "medium".into() } else { a.priority.clone() },
                explanation: a.explanation.clone(),
                confidence: if a.confidence.is_empty() { "confirm".into() } else { a.confidence.clone() },
            }),
        });
    }
    out
}

/// 调用 OpenAI 兼容的 chat/completions 接口
fn call_llm(prompt: &str, api_key: &str) -> Result<String, String> {
    let base = std::env::var("QTTCODE_LLM_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = std::env::var("QTTCODE_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "temperature": 0,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": prompt},
        ],
    });

    let resp = ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .timeout(std::time::Duration::from_secs(120))
        .send_json(body)
        .map_err(|e| format!("LLM 调用失败: {}", e))?;

    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("LLM 响应解析失败: {}", e))?;
    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "LLM 响应缺少 choices[0].message.content".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::Severity;
    use std::path::PathBuf;

    fn finding(rule: &str, line: usize, message: &str) -> Finding {
        Finding {
            file_path: PathBuf::from("src/main.rs"),
            line,
            column: 1,
            severity: Severity::Should,
            rule_id: rule.to_string(),
            message: message.to_string(),
        }
    }

    #[test]
    fn test_lint_mode_no_llm() {
        let findings = vec![finding("long-function", 53, "函数过长")];
        let out = run_llm_stage("lint", &findings).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].llm.is_none());
        assert_eq!(out[0].message, "函数过长");
    }

    #[test]
    fn test_llm_mode_without_key_falls_back() {
        // 确保环境变量不存在
        unsafe { std::env::remove_var("QTTCODE_LLM_API_KEY") };
        let findings = vec![finding("long-function", 53, "函数过长")];
        let out = run_llm_stage("llm", &findings).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].llm.is_none(), "未配置 LLM 应回退 lint");
    }

    #[test]
    fn test_deep_mode_without_key_falls_back() {
        unsafe { std::env::remove_var("QTTCODE_LLM_API_KEY") };
        let findings = vec![finding("long-function", 53, "函数过长")];
        let out = run_llm_stage("deep", &findings).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].llm.is_none());
    }

    #[test]
    fn test_unknown_mode_errors() {
        let err = run_llm_stage("bogus", &[]).unwrap_err();
        assert!(err.contains("未知 mode"));
    }

    #[test]
    fn test_build_prompt_shape() {
        let findings = vec![finding("long-function", 53, "函数 `run` 共 90 行")];
        let prompt = build_prompt(&findings);
        let v: serde_json::Value = serde_json::from_str(&prompt).unwrap();
        assert_eq!(v[0]["rule_id"], "long-function");
        assert_eq!(v[0]["line"], 53);
        assert_eq!(v[0]["severity"], "SHOULD");
    }

    #[test]
    fn test_parse_llm_response_plain_json() {
        let content = r#"[{"file":"src/main.rs","line":53,"rule_id":"long-function","priority":"high","explanation":"拆分为两个函数","confidence":"confirm"}]"#;
        let anns = parse_llm_response(content);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].priority, "high");
        assert_eq!(anns[0].confidence, "confirm");
        assert!(!anns[0].semantic);
    }

    #[test]
    fn test_parse_llm_response_markdown_fence() {
        let content = "```json\n[{\"file\":\"a.rs\",\"line\":1,\"rule_id\":\"r\",\"explanation\":\"x\"}]\n```";
        let anns = parse_llm_response(content);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].rule_id, "r");
    }

    #[test]
    fn test_parse_llm_response_invalid_returns_empty() {
        assert!(parse_llm_response("not json at all").is_empty());
        assert!(parse_llm_response("{\"object\": true}").is_empty());
    }

    #[test]
    fn test_merge_annotations() {
        let findings = vec![finding("long-function", 53, "函数 `run` 共 90 行")];
        let anns = vec![LlmAnnotation {
            file: "src/main.rs".into(),
            line: 53,
            rule_id: "long-function".into(),
            priority: "high".into(),
            explanation: "拆分为 resolve_config 和 scan_files".into(),
            confidence: "confirm".into(),
            semantic: false,
            message: String::new(),
            severity: String::new(),
        }];
        let out = merge(&findings, &anns);
        assert_eq!(out.len(), 1);
        let llm = out[0].llm.as_ref().unwrap();
        assert_eq!(llm.priority, "high");
        assert_eq!(llm.confidence, "confirm");
        assert_eq!(out[0].message, "函数 `run` 共 90 行", "原始信息不变");
    }

    #[test]
    fn test_merge_annotation_mismatch_keeps_plain() {
        let findings = vec![finding("long-function", 53, "函数 `run` 共 90 行")];
        let anns = vec![LlmAnnotation {
            file: "src/main.rs".into(),
            line: 99, // 行不匹配
            rule_id: "long-function".into(),
            priority: "high".into(),
            explanation: "x".into(),
            confidence: "confirm".into(),
            semantic: false,
            message: String::new(),
            severity: String::new(),
        }];
        let out = merge(&findings, &anns);
        assert!(out[0].llm.is_none());
    }

    #[test]
    fn test_merge_semantic_findings_appended() {
        let findings = vec![finding("long-function", 53, "函数 `run` 共 90 行")];
        let anns = vec![LlmAnnotation {
            file: "src/api/handler.rs".into(),
            line: 12,
            rule_id: "llm-semantic".into(),
            priority: "high".into(),
            explanation: "未校验用户输入，存在注入风险".into(),
            confidence: "confirm".into(),
            semantic: true,
            message: "SQL 注入风险".into(),
            severity: "MUST".into(),
        }];
        let out = merge(&findings, &anns);
        assert_eq!(out.len(), 2);
        let sem = &out[1];
        assert_eq!(sem.rule_id, "llm-semantic");
        assert_eq!(sem.severity, "MUST");
        assert_eq!(sem.message, "SQL 注入风险");
        assert!(sem.llm.is_some());
    }
}

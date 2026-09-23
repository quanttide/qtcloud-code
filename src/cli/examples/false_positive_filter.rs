//! false_positive_filter — 假阳性过滤：规则引擎 findings 交 LLM 复核，统计驳回率
//!
//! 由实验室 `llm_exp` 改写：findings 经 lib 的 review 扫描管线收集（不再依赖
//! 已安装的 CLI），目标目录参数化（默认当前目录），报告输出到标准输出，
//! 未配置 LLM 时跳过。
//!
//! 运行：`LLM_API_KEY=sk-... cargo run --example false_positive_filter [-- <目标目录>]`

use std::path::Path;

const SYSTEM: &str = "你是一个专业的代码审查助手，回答格式为 JSON。";
const MAX_VERIFIED: usize = 10;

fn main() {
    let api_key = match qtcloud_code_cli::llm::get_api_key() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("未配置 LLM_API_KEY，跳过 LLM 复核");
            return;
        }
    };

    let target = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let root = match Path::new(&target).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("路径不存在: {} ({})", target, e);
            return;
        }
    };

    // 1. 经 lib 的 review 扫描管线收集 findings
    let rules = Some(vec![
        "long-function".to_string(),
        "long-parameter-list".to_string(),
        "unused-variable".to_string(),
        "missing-tests".to_string(),
    ]);
    let findings = match qtcloud_code_cli::review::collect_findings(&root, &rules) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("review 失败: {}", e);
            return;
        }
    };
    println!("total findings: {}", findings.len());

    let sample: Vec<_> = findings.iter().take(MAX_VERIFIED).collect();
    if findings.len() > MAX_VERIFIED {
        println!("（仅复核前 {} 条，避免批量 LLM 调用）", MAX_VERIFIED);
    }

    // 2. 逐条交 LLM 判定 confirm / dismiss
    let mut results = Vec::new();
    for (i, f) in sample.iter().enumerate() {
        let file = f.file_path.to_string_lossy().to_string();
        let ctx = read_context(&f.file_path, f.line);
        let prompt = format!(
            "以下是一个代码审查 finding。请判断它是否为真问题：\n\n\
             文件: {}:{}\n规则: {}\n消息: {}\n\n\
             代码上下文:\n```\n{}\n```\n\n\
             请返回 JSON:\n{{\"verdict\": \"confirm/dismiss\", \"reason\": \"...\"}}",
            file, f.line, f.rule_id, f.message, ctx
        );
        match qtcloud_code_cli::llm::call_llm(SYSTEM, &prompt, &api_key) {
            Ok(resp) => {
                let v: serde_json::Value =
                    serde_json::from_str(strip_fence(&resp)).unwrap_or(serde_json::Value::Null);
                let verdict = if v["verdict"] == "dismiss" {
                    "DISMISS"
                } else {
                    "CONFIRM"
                };
                let reason = v["reason"].as_str().unwrap_or("").to_string();
                println!(
                    "  [{}/{}] {}:{} [{}] → {}",
                    i + 1,
                    sample.len(),
                    file,
                    f.line,
                    verdict,
                    reason.lines().next().unwrap_or("")
                );
                results.push((file, f.line, f.rule_id.clone(), verdict.to_string(), reason));
            }
            Err(e) => eprintln!("  [{}/{}] LLM 错误: {}", i + 1, sample.len(), e),
        }
    }

    // 3. 统计报告（输出到标准输出）
    let total = results.len();
    let dismiss = results.iter().filter(|r| r.3 == "DISMISS").count();
    let confirm = results.iter().filter(|r| r.3 == "CONFIRM").count();
    let rate = if total > 0 {
        dismiss as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    println!("\n# 假阳性复核报告");
    println!("\n| 指标 | 值 |");
    println!("|------|---|");
    println!("| 总 finding 数 | {} |", findings.len());
    println!("| 参与复核 | {} |", total);
    println!("| LLM 确认 (CONFIRM) | {} |", confirm);
    println!("| LLM 驳回 (DISMISS) | {} |", dismiss);
    println!("| 驳回率 | {:.1}% |", rate);

    let dismissed: Vec<String> = results
        .iter()
        .filter(|r| r.3 == "DISMISS")
        .map(|r| {
            format!(
                "- `{}:{}` [{}] {}",
                r.0,
                r.1,
                r.2,
                r.4.lines().next().unwrap_or("")
            )
        })
        .collect();
    if !dismissed.is_empty() {
        println!("\n## 驳回详情\n");
        for line in &dismissed {
            println!("{}", line);
        }
    }
}

/// 取文件中目标行 ±5 行作为上下文
fn read_context(file: &Path, line: usize) -> String {
    let content = std::fs::read_to_string(file).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    let start = if line > 5 { line - 5 } else { 1 };
    let end = (line + 5).min(lines.len());
    (start..=end)
        .filter_map(|i| {
            let marker = if i == line { " →" } else { "  " };
            lines.get(i - 1).map(|l| format!("{}{}: {}", marker, i, l))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 去除响应外层的 markdown 代码块
fn strip_fence(s: &str) -> &str {
    let t = s.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    t.strip_suffix("```").unwrap_or(t).trim()
}

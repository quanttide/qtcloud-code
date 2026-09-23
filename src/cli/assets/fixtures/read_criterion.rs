// 自 qtcloud-work 抽取的案例（原 src/criterion/read.rs 的 read_criterion）
//! 判据的读法：从定义里的字段读出 [`Criterion`]，顺带把语法过一遍。
//!
//! 判据是**字段**，不是一行小语法：`path` 存在、`absent` 不存在、
//! `file` + `contains` 含这段文字、`run` 这条命令退出码为零。字段名、取值、
//! 不认识、缺了、越界，当场报错。规矩的出处是 `docs/specification/process/workflow.md`·语法。

/// 读一条判据：取值不对、缺该有的字段，当场报错。
///
/// `step` 与 `order` 是这条判据的位置（第几个步骤、第几条判据），只用来说话；
/// 返回的是认好的值对象。
pub fn read_criterion(
    value: &Yaml,
    step: usize,
    order: usize,
) -> Result<Criterion, DefinitionError> {
    let at = || Position::Criterion {
        step,
        criterion: order,
    };
    // 先看是不是映射：不是映射时要说「不是映射」，不能先说 executor 该怎么写——
    // 那样报错会指错方向（2026-09-12 之前正是这个顺序，这条分支因此永远走不到）。
    let criterion_map = value
        .as_mapping()
        .ok_or_else(|| DefinitionError::new(at(), Fault::CriterionNotMapping))?;
    let kind = text_of(value, "executor");
    if !CRITERION_TYPES.contains(&kind.as_str()) {
        return Err(DefinitionError::new(at(), Fault::BadCriterionExecutor));
    }
    let odd = unknown_fields(criterion_map, &CRITERION_FIELDS);
    if !odd.is_empty() {
        return Err(DefinitionError::new(
            at(),
            Fault::UnknownCriterionFields(odd),
        ));
    }
    let given: Vec<&str> = ["path", "absent", "file", "contains", "run"]
        .into_iter()
        .filter(|name| value.get(*name).is_some())
        .collect();
    if kind == RULE {
        if given.is_empty() {
            return Err(DefinitionError::new(at(), Fault::RuleNeedsJudgement));
        }
        if given.contains(&"contains") && !given.contains(&"file") {
            return Err(DefinitionError::new(at(), Fault::ContainsNeedsFile));
        }
        if given.contains(&"file") && !given.contains(&"contains") {
            return Err(DefinitionError::new(at(), Fault::FileNeedsContains));
        }
        let others: Vec<&str> = given
            .iter()
            .copied()
            .filter(|name| *name != "file" && *name != "contains")
            .collect();
        if others.len() > 1 || (!others.is_empty() && given.contains(&"file")) {
            return Err(DefinitionError::new(at(), Fault::OnlyOneJudgement));
        }
    } else {
        if text_of(value, "description").is_empty() {
            return Err(DefinitionError::new(at(), Fault::NeedsDescription { kind }));
        }
        if !given.is_empty() {
            return Err(DefinitionError::new(
                at(),
                Fault::NoRuleFields {
                    kind,
                    given: given.iter().map(|name| name.to_string()).collect(),
                },
            ));
        }
    }
    // 路径里只认四个占位；写别的（如 `{{foo}}`）算不合语法（规范 `process/workflow.md`·语法）。
    let mut unknown: Vec<String> = Vec::new();
    for field in ["path", "absent", "file"] {
        for name in placeholders_in(&text_of(value, field)) {
            if !PLACEHOLDER_NAMES.contains(&name.as_str()) && !unknown.contains(&name) {
                unknown.push(name);
            }
        }
    }
    if !unknown.is_empty() {
        return Err(DefinitionError::new(
            at(),
            Fault::UnknownPlaceholder(unknown),
        ));
    }
    Ok(criterion_of(value))
}

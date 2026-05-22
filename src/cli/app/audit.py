"""代码审计逻辑：封装 ruff 和 lizard 的 CLI 调用。"""

import subprocess
import sys
from dataclasses import dataclass, field


@dataclass
class AuditResult:
    source: str
    ruff_errors: int = 0
    ruff_output: str = ""
    lizard_output: str = ""
    format_issues: int = 0
    format_output: str = ""
    warnings: list[str] = field(default_factory=list)
    passed: bool = True

    def summary(self) -> str:
        lines = [
            f"## 审计报告：{self.source}",
            "",
            "### 1. 代码规范",
            f"- ruff 错误数: {self.ruff_errors}",
            "",
            "### 2. 格式",
            f"- 格式问题文件数: {self.format_issues}",
            "",
            "### 3. 圈复杂度",
        ]
        if self.lizard_output:
            for line in self.lizard_output.splitlines():
                if "!!!! Warnings" in line or "NLOC" in line or "Total" in line:
                    lines.append(f"- {line.strip()}")
            for w in self.warnings:
                lines.append(f"- ⚠ {w}")
        else:
            lines.append("- lizard 未执行")
        lines.append("")
        verdict = "✅ 通过" if self.passed else "❌ 需改进"
        lines.append(f"### 4. 总体评估: {verdict}")
        return "\n".join(lines)


def run_ruff_check(source: str) -> tuple[int, str]:
    result = subprocess.run(
        [sys.executable, "-m", "ruff", "check", source],
        capture_output=True,
        text=True,
    )
    # count actual error lines (non-empty, non-summary)
    errors = result.stdout if result.returncode != 0 else ""
    lines = [
        ln for ln in errors.splitlines() if ln.strip() and not ln.startswith("Found")
    ]
    return len(lines), (errors + result.stderr).strip()


def run_ruff_format(source: str) -> tuple[int, str]:
    result = subprocess.run(
        [sys.executable, "-m", "ruff", "format", "--check", source],
        capture_output=True,
        text=True,
    )
    output = (result.stdout + result.stderr).strip()
    # count "Would reformat" lines
    count = output.count("Would reformat")
    return count, output


def run_lizard(source: str) -> str:
    result = subprocess.run(
        ["lizard", "--languages", "python", source],
        capture_output=True,
        text=True,
    )
    return (result.stdout + result.stderr).strip()


def parse_lizard_warnings(lizard_output: str) -> list[str]:
    warnings = []
    capture = False
    for line in lizard_output.splitlines():
        if "!!!! Warnings" in line:
            capture = True
            continue
        if capture and line.strip() and not line.startswith("="):
            parts = line.split()
            if len(parts) >= 5:
                try:
                    ccn = int(parts[1])
                    if ccn > 15:
                        location = parts[-1] if len(parts) > 5 else "unknown"
                        warnings.append(f"CCN={ccn} at {location}")
                except ValueError:
                    continue
    return warnings


def run(source: str) -> AuditResult:
    result = AuditResult(source=source)

    # 1. ruff check
    err_count, ruff_out = run_ruff_check(source)
    result.ruff_errors = err_count
    result.ruff_output = ruff_out
    if err_count > 10:
        result.passed = False

    # 2. ruff format
    fmt_count, fmt_out = run_ruff_format(source)
    result.format_issues = fmt_count
    result.format_output = fmt_out
    if fmt_count > 0:
        result.passed = False

    # 3. lizard
    try:
        lizard_out = run_lizard(source)
        result.lizard_output = lizard_out
        result.warnings = parse_lizard_warnings(lizard_out)
        if any("CCN=" in w for w in result.warnings):
            result.passed = False
    except FileNotFoundError:
        result.warnings.append("lizard 未安装，跳过圈复杂度分析")

    return result

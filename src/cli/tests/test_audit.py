from app.audit import AuditResult, parse_lizard_warnings, run


def test_audit_result_summary():
    r = AuditResult(source="tests", ruff_errors=0, passed=True)
    summary = r.summary()
    assert "审计报告" in summary
    assert "✅ 通过" in summary


def test_audit_result_summary_failed():
    r = AuditResult(source="tests", ruff_errors=15, passed=False)
    summary = r.summary()
    assert "❌ 需改进" in summary


def test_parse_lizard_warnings_empty():
    assert parse_lizard_warnings("No warnings") == []


def test_parse_lizard_warnings_found():
    output = """!!!! Warnings (cyclomatic_complexity > 15) !!!!
    134     46    722      2     165 run@127-291@path/to/file.py"""
    warnings = parse_lizard_warnings(output)
    assert len(warnings) == 1
    assert "CCN=46" in warnings[0]


def test_run_ruff_check(monkeypatch):
    from app import audit

    def mock_run(*args, **kw):
        class Result:
            returncode = 0
            stdout = ""
            stderr = ""

        return Result()

    monkeypatch.setattr(audit.subprocess, "run", mock_run)
    result = run("tests")
    assert result.ruff_errors == 0


def test_run_lizard_missing(monkeypatch):
    from app import audit

    def mock_run(*args, **kw):
        raise FileNotFoundError("lizard not found")

    def mock_ruff(*a):
        return 0, ""

    monkeypatch.setattr(audit, "run_ruff_check", mock_ruff)
    monkeypatch.setattr(audit, "run_ruff_format", lambda s: (0, ""))
    monkeypatch.setattr(audit.subprocess, "run", mock_run)

    result = run("tests")
    assert any("lizard 未安装" in w for w in result.warnings)

from pathlib import Path

from typer.testing import CliRunner

from app.cli import app

runner = CliRunner()


def test_help():
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0


def test_main_help(monkeypatch):
    from app.cli import main

    monkeypatch.setattr("sys.argv", ["app.cli", "--help"])
    try:
        main()
    except SystemExit as e:
        assert e.code == 0


def test_main_module_entry():
    import subprocess
    import sys

    root = Path(__file__).resolve().parent.parent
    result = subprocess.run(
        [sys.executable, "-m", "app.cli", "--help"],
        capture_output=True,
        text=True,
        cwd=str(root),
    )
    assert result.returncode == 0

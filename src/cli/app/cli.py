#!/usr/bin/env python3
"""qtcloud-code-cli — 代码审计与质量管理命令行工具。"""

import typer

app = typer.Typer()


@app.callback()
def main_callback() -> None: ...


@app.command()
def audit(
    source: str = typer.Argument(
        ".",
        help="源码目录路径",
    ),
):
    """对源码目录执行代码审计（ruff + lizard）。

    依次执行 ruff check、ruff format --check、lizard，
    输出包含代码规范错误、格式问题和圈复杂度分析的审计报告。
    """
    from app.audit import run

    result = run(source)
    print(result.summary())
    raise typer.Exit(code=0 if result.passed else 1)


def main():
    return app()


if __name__ == "__main__":  # pragma: no cover
    exit(main())

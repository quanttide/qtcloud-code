#!/usr/bin/env python3
"""qtcloud-code-cli — 代码审计与质量管理命令行工具。"""

import typer

app = typer.Typer()


@app.callback()
def main_callback() -> None: ...


def main():
    return app()


if __name__ == "__main__":  # pragma: no cover
    exit(main())

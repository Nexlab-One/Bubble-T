#!/usr/bin/env python3
"""Fix View::new(...); missing closing paren in examples."""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"


def balance_fix(line: str) -> str:
    m = re.match(r"^(\s*)let mut view = View::new\((.+);\s*$", line)
    if not m:
        m = re.match(r"^(\s*)let mut view = View::new\((.+);\s*$", line.replace("let view", "let mut view"))
    if not m:
        m = re.match(r"^(\s*)let mut view = View::new\((.+);\s*$", line)
    if m:
        indent, expr = m.group(1), m.group(2)
        open_p = expr.count("(")
        close_p = expr.count(")")
        if open_p > close_p:
            expr += ")" * (open_p - close_p)
        return f"{indent}let mut view = View::new({expr});"
    m = re.match(r"^(\s*)let bar = View::new\((.+);\s*$", line)
    if m:
        indent, expr = m.group(1), m.group(2)
        open_p = expr.count("(")
        close_p = expr.count(")")
        if open_p > close_p:
            expr += ")" * (open_p - close_p)
        return f"{indent}let bar = {expr};"  # bar should be String not View - handled separately
    return line


def fix_file(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    original = text

    # View::new(expr; -> View::new(expr); with balanced parens
    def fix_view_new_line(match: re.Match[str]) -> str:
        prefix = match.group(1)
        expr = match.group(2)
        open_p = expr.count("(")
        close_p = expr.count(")")
        if open_p > close_p:
            expr += ")" * (open_p - close_p)
        return f"{prefix}View::new({expr});"

    text = re.sub(
        r"^(\s*(?:let mut view|let bar) = )?View::new\(([^;\n]+);\s*$",
        fix_view_new_line,
        text,
        flags=re.MULTILINE,
    )

    # Bare format!(...) at end of view fn -> View::new(format!(...))
    text = re.sub(
        r"(\n        )(format!\([\s\S]*?\)\))\s*\n(\s*\})\s*\n(\})\s*\n\n",
        r"\1View::new\2\n\3\n\4\n\n",
        text,
    )

    if text != original:
        path.write_text(text, encoding="utf-8")
        return True
    return False


def main() -> None:
    for path in sorted(EXAMPLES.rglob("*.rs")):
        if fix_file(path):
            print(path.relative_to(ROOT))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Wrap doctest view bodies with View::new in doc comment lines only."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WIDGETS = ROOT / "crates" / "bubble-t-widgets" / "src"


def fix_doc_line_view_returns(text: str) -> str:
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        prefix_match = re.match(r"^(\s*(?://!|///)\s*)", line)
        if prefix_match and "fn view(&self) -> View {" in line:
            out.append(line)
            i += 1
            body_lines: list[str] = []
            while i < len(lines):
                bl = lines[i]
                if not re.match(r"^\s*(?://!|///)", bl):
                    body_lines.append(bl)
                    out.extend(body_lines)
                    break
                if re.search(r"fn view\(&self\) -> View \{\s*$", bl) or (
                    "fn view(&self) -> View {" in bl and "{" in bl.split("View {")[-1]
                ):
                    out.append(bl)
                    i += 1
                    continue
                stripped = re.sub(r"^\s*(?://!|///)\s?", "", bl)
                if stripped.startswith("}"):
                    # wrap accumulated body
                    if body_lines:
                        joined = "".join(
                            re.sub(r"^\s*(?://!|///)\s?", "", x) for x in body_lines
                        ).strip()
                        if joined and not joined.startswith("View::new("):
                            indent = re.match(r"^(\s*(?://!|///)\s*)", body_lines[0]).group(1)
                            if joined.startswith("format!") or joined.startswith("join_"):
                                out.append(f"{indent}View::new({joined})\n")
                            elif joined.startswith("self.") and joined.endswith(".view()"):
                                out.append(f"{indent}View::new({joined})\n")
                            else:
                                out.extend(body_lines)
                        else:
                            out.extend(body_lines)
                        body_lines = []
                    out.append(bl)
                    i += 1
                    break
                body_lines.append(bl)
                i += 1
            continue

        # single-line view body: //!         format!(...)
        m = re.match(
            r"^(\s*(?://!|///)\s*)(format!\(.+\))\s*$",
            line.rstrip("\n"),
        )
        if m and i > 0 and "fn view(&self) -> View" in "".join(out[-5:]):
            out.append(f"{m.group(1)}View::new({m.group(2)})\n")
            i += 1
            continue

        out.append(line)
        i += 1
    return "".join(out)


def fix_file(path: Path) -> bool:
    original = path.read_text(encoding="utf-8")
    text = original

    # Direct replacements in doc lines
    text = re.sub(
        r"^(\\s*(?://!|///)\\s*)format!(\"\\{\\} Loading\\.\\.\\.\", self\\.spinner\\.view\\(\\))\\s*$",
        r"\\1View::new(format!(\"{} Loading...\", self.spinner.view()))",
        text,
        flags=re.MULTILINE,
    )
    text = re.sub(
        r"^(\\s*(?://!|///)\\s*)format!(\"Enter text: \\{\\}\\\\n\\{\\}\", self\\.input\\.view\\(\\), \"Press Ctrl\\+C to quit\")\\s*$",
        r"\\1View::new(format!(\"Enter text: {}\\n{}\", self.input.view(), \"Press Ctrl+C to quit\"))",
        text,
        flags=re.MULTILINE,
    )

    patterns = [
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"\\{\\}\\\\n\\{\\}\", self\\.input\\.view\\(\\), self\\.list\\.view\\(\\)\\.content)\\s*$",
            r"\\1View::new(format!(\"{}\\n{}\", self.input.view(), self.list.view().content))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"Loading: \\{\\}\\\\n\", self\\.progress\\.view\\(\\))\\s*$",
            r"\\1View::new(format!(\"Loading: {}\\n\", self.progress.view()))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"Elapsed time: \\{\\}\", self\\.stopwatch\\.view\\(\\))\\s*$",
            r"\\1View::new(format!(\"Elapsed time: {}\", self.stopwatch.view()))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"Time remaining: \\{\\}\", self\\.timer\\.view\\(\\))\\s*$",
            r"\\1View::new(format!(\"Time remaining: {}\", self.timer.view()))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"\\{\\} - Remaining: \\{\\}\", self\\.status, self\\.timer\\.view\\(\\))\\s*$",
            r"\\1View::new(format!(\"{} - Remaining: {}\", self.status, self.timer.view()))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"Document Viewer\\\\n\\\\n\\{\\}\\\\n\\\\nScroll: \\{:.1\\}%\", self\\.viewport\\.view\\(\\)\\.content, self\\.viewport\\.scroll_percent\\(\\))\\s*$",
            r"\\1View::new(format!(\"Document Viewer\\n\\n{}\\n\\nScroll: {:.1}%\", self.viewport.view().content, self.viewport.scroll_percent()))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"My Data Table:\\\\n\\\\n\\{\\}\", self\\.table\\.view\\(\\))\\s*$",
            r"\\1View::new(format!(\"My Data Table:\\n\\n{}\", self.table.view()))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"My Application\\\\n\\\\n\\{\\}\", self\\.table\\.view\\(\\)\\.content)\\s*$",
            r"\\1View::new(format!(\"My Application\\n\\n{}\", self.table.view().content))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"\\{\\}\\\\n\\{\\}\", \"Your app content here\", help_view)\\s*$",
            r"\\1View::new(format!(\"{}\\n{}\", \"Your app content here\", help_view))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"Timer: \\{\\}\", self\\.stopwatch\\.view\\(\\))\\s*$",
            r"\\1View::new(format!(\"Timer: {}\", self.stopwatch.view()))",
        ),
        (
            r"^(\\s*(?://!|///)\\s*)format!(\"\\{\\} - Remaining: \\{\\}\", self\\.status, self\\.timer\\.view\\(\\))\\s*$",
            r"\\1View::new(format!(\"{} - Remaining: {}\", self.status, self.timer.view()))",
        ),
    ]

    for pat, repl in patterns:
        text = re.sub(pat, repl, text, flags=re.MULTILINE)

    # Generic: doc line with only format! in view fn
    text = re.sub(
        r"^(\\s*(?://!|///)\\s*)(format!\([\\s\\S]*?\))\\s*$",
        lambda m: f"{m.group(1)}View::new({m.group(2)})"
        if "View::new(" not in m.group(2)
        else m.group(0),
        text,
        flags=re.MULTILINE,
    )

    # Hidden doctest one-liners
    text = text.replace(
        "#   fn view(&self) -> View { self.spinner.view() }",
        "#   fn view(&self) -> View { View::new(self.spinner.view()) }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { self.stopwatch.view() }",
        "#   fn view(&self) -> View { View::new(self.stopwatch.view()) }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { self.timer.view() }",
        "#   fn view(&self) -> View { View::new(self.timer.view()) }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { self.progress.view() }",
        "#   fn view(&self) -> View { View::new(self.progress.view()) }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { self.viewport.view().content }",
        "#   fn view(&self) -> View { View::new(self.viewport.view().content) }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { self.table.view().content }",
        "#   fn view(&self) -> View { View::new(self.table.view().content) }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { format!(\"My Application\\n\\n{}\", self.table.view().content) }",
        "#   fn view(&self) -> View { View::new(format!(\"My Application\\n\\n{}\", self.table.view().content)) }",
    )

    if text != original:
        path.write_text(text, encoding="utf-8")
        return True
    return False


def main() -> None:
    changed = []
    for path in sorted(WIDGETS.rglob("*.rs")):
        if fix_file(path):
            changed.append(str(path.relative_to(ROOT)))
    print(f"Updated {len(changed)} files")


if __name__ == "__main__":
    main()

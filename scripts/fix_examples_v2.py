#!/usr/bin/env python3
"""Fix common v2 View migration mistakes in examples."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"

# Widget types whose BubbleTeaModel::view() returns View (use .content in format strings).
VIEW_WIDGETS = {
    "list",
    "viewport",
    "filepicker",
    "table",  # table Model trait view returns View; inherent pub fn view returns String - trait when ambiguous
}

# Fix malformed: format!( ... View::new())  ->  View::new(format!( ... ))
FORMAT_VIEW_NEW_RE = re.compile(
    r"(\s*)((?:format!|format)\!\(\s*[\s\S]*?)\s*View::new\(\)\)",
    re.MULTILINE,
)


def fix_format_view_new(text: str) -> str:
    def repl(m: re.Match[str]) -> str:
        indent = m.group(1)
        fmt = m.group(2).rstrip()
        return f"{indent}View::new({fmt}))"

    prev = None
    while prev != text:
        prev = text
        text = FORMAT_VIEW_NEW_RE.sub(repl, text)
    return text


def fix_broken_view_new_format(text: str) -> str:
    """Fix View::new(format!(...expr; missing paren before semicolon."""
    # View::new(format!("...", self.foo.view(); -> View::new(format!("...", self.foo.view().content));
    text = re.sub(
        r"View::new\(format!\(([\s\S]*?)(\w+)\.view\(\)\s*;",
        r"View::new(format!(\1\2.view().content);",
        text,
    )
    # View::new(format!("...", self.foo.view() without closing - add .content and ))
    text = re.sub(
        r"View::new\(format!\(([\s\S]*?)(\w+)\.view\(\)\s*\n",
        lambda m: f"View::new(format!({m.group(1)}{m.group(2)}.view().content))\n",
        text,
    )
    return text


def fix_return_string_in_view(text: str) -> str:
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    in_view = False
    depth = 0
    for line in lines:
        if re.search(r"\bfn view\(&self\)\s*->\s*View\b", line):
            in_view = True
            depth = 0
        if in_view:
            depth += line.count("{") - line.count("}")
            if depth < 0:
                in_view = False
            stripped = line.lstrip()
            indent = line[: len(line) - len(stripped)]
            if stripped.startswith("return String::new();"):
                line = f"{indent}return View::new(\"\");\n"
            elif stripped.startswith('return String::from("");'):
                line = f'{indent}return View::new("");\n'
            elif stripped.startswith('return String::from(\'\');'):
                line = f"{indent}return View::new(\"\");\n"
            elif m := re.match(
                r'return "([^"]*)"\s*\.to_string\(\)\s*;', stripped
            ):
                line = f'{indent}return View::new("{m.group(1)}");\n'
            elif m := re.match(r'return "([^"]*)"\s*;', stripped):
                line = f'{indent}return View::new("{m.group(1)}");\n'
            elif m := re.match(
                r"return (.+\.render\([^)]*\))\s*;", stripped
            ):
                line = f"{indent}return View::new({m.group(1)});\n"
            if depth == 0 and "}" in line and "fn view" not in line:
                in_view = False
        out.append(line)
    return "".join(out)


def fix_initializing_return(text: str) -> str:
    return re.sub(
        r'return "\\n  Initializing\.\.\."\.to_string\(\);',
        'return View::new("\\n  Initializing...");',
        text,
    )


def remove_builder_options(text: str) -> str:
    text = re.sub(r"\s*\.alt_screen\([^)]*\)\s*", "\n", text)
    text = re.sub(r"\s*\.report_focus\([^)]*\)\s*", "\n", text)
    text = re.sub(
        r"\s*\.mouse_motion\([^)]*\)\s*//[^\n]*\n",
        "\n",
        text,
    )
    text = re.sub(r"\s*\.mouse_motion\([^)]*\)\s*\n", "\n", text)
    return text


def fix_widget_view_in_format(text: str) -> str:
    for w in VIEW_WIDGETS:
        text = re.sub(
            rf"(\bself\.{w}\.view\(\))(?!\.content)",
            r"\1.content",
            text,
        )
    # filepicker nested
    text = re.sub(
        r"push_str\(&self\.filepicker\.view\(\)\)",
        "push_str(&self.filepicker.view().content)",
        text,
    )
    text = re.sub(
        r"push_str\(&self\.viewport\.view\(\)\)",
        "push_str(&self.viewport.view().content)",
        text,
    )
    return text


def fix_test_dots(text: str) -> str:
    text = re.sub(
        r'println!\("\{\}", (\w+)\.view\(\)\);',
        r'println!("{}", \1.view().content);',
        text,
    )
    text = re.sub(
        r"println!\(\"Paginator with dots: '\{\}'\", paginator\.view\(\)\);",
        'println!("Paginator with dots: \'{}\'", paginator.view());',
        text,
    )
    return text


def process_file(path: Path) -> bool:
    original = path.read_text(encoding="utf-8")
    text = original
    text = fix_format_view_new(text)
    text = fix_broken_view_new_format(text)
    text = fix_return_string_in_view(text)
    text = fix_initializing_return(text)
    if path.suffix == ".rs":
        text = remove_builder_options(text)
        text = fix_widget_view_in_format(text)
    if path.name == "test_dots.rs":
        text = fix_test_dots(text)
    if text != original:
        path.write_text(text, encoding="utf-8")
        return True
    return False


def main() -> None:
    changed: list[str] = []
    for path in sorted(EXAMPLES.rglob("*.rs")):
        if process_file(path):
            changed.append(str(path.relative_to(ROOT)))
    print(f"Updated {len(changed)} files:")
    for p in changed:
        print(f"  {p}")


if __name__ == "__main__":
    main()

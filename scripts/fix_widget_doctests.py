#!/usr/bin/env python3
"""Update bubble-t-widgets doc examples from String view to View API."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WIDGETS = ROOT / "crates" / "bubble-t-widgets" / "src"

IMPORT_FIXES = [
    (
        "use bubble_t::{Model as BubbleTeaModel, Msg}",
        "use bubble_t::{Model as BubbleTeaModel, Msg, View}",
    ),
    (
        "use bubble_t::{Model as BubbleTeaModel, Msg, Cmd}",
        "use bubble_t::{Model as BubbleTeaModel, Msg, Cmd, View}",
    ),
    (
        "use bubble_t::{Model, Msg}",
        "use bubble_t::{Model, Msg, View}",
    ),
    (
        "use bubble_t::{Model, Cmd, Msg}",
        "use bubble_t::{Model, Cmd, Msg, View}",
    ),
    (
        "use bubble_t::{Cmd, Model, Msg}",
        "use bubble_t::{Cmd, Model, Msg, View}",
    ),
    (
        "use bubble_t::Model as BubbleTeaModel;",
        "use bubble_t::{Model as BubbleTeaModel, View};",
    ),
]


def fix_file(path: Path) -> bool:
    original = path.read_text(encoding="utf-8")
    text = original

    text = text.replace("fn view(&self) -> String", "fn view(&self) -> View")
    text = text.replace("#   fn view(&self) -> String", "#   fn view(&self) -> View")
    text = text.replace("return String::new()", "return View::new(\"\")")
    text = text.replace("#   return String::new()", "#   return View::new(\"\")")
    text = text.replace("unimplemented!()", "View::new(\"\")")
    text = text.replace("self.viewport.view()", "self.viewport.view().content")
    text = text.replace("self.table.view()", "self.table.view().content")
    text = text.replace("m.list.view().contains", "m.list.view().content.contains")
    text = text.replace(
        "#   fn view(&self) -> View { self.viewport.view() }",
        "#   fn view(&self) -> View { self.viewport.view() }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { self.timer.view() }",
        "#   fn view(&self) -> View { View::new(self.timer.view()) }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { self.work_timer.view() }",
        "#   fn view(&self) -> View { View::new(self.work_timer.view()) }",
    )
    text = text.replace(
        "#   fn view(&self) -> View { format!(\"Timer: {} {}\", self.timer.view(), if self.paused { \"(PAUSED)\" } else { \"\" }) }",
        "#   fn view(&self) -> View { View::new(format!(\"Timer: {} {}\", self.timer.view(), if self.paused { \"(PAUSED)\" } else { \"\" })) }",
    )

    for old, new in IMPORT_FIXES:
        if old in text and new not in text:
            text = text.replace(old, new)

    # Wrap format! returns in view() that aren't already wrapped
    text = re.sub(
        r"(fn view\(&self\) -> View \{\n\s+)format!",
        r"\1View::new(format!",
        text,
    )
    text = re.sub(
        r"(#   fn view\(&self\) -> View \{\n\s+)format!",
        r"\1View::new(format!",
        text,
    )
    # Add closing paren for View::new(format!(...)) single-statement views
    text = re.sub(
        r"(View::new\(format!\([\s\S]*?\))\s*\n(\s*\})",
        r"\1)\n\2",
        text,
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
    print(f"Updated {len(changed)} files:")
    for p in changed:
        print(f"  {p}")


if __name__ == "__main__":
    main()

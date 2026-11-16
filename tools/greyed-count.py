#!/usr/bin/env python3
"""Counts the greyed items listed in docs/tasks/greyed-items.md.

Prints the totals (menu bar, toolbars, context menus) and the number of items per package tag.
Exits with status 1 when an item line has no package tag, so the list cannot drift untagged.

Usage: python tools/greyed-count.py [path/to/greyed-items.md]
"""

import re
import sys
from pathlib import Path

DEFAULT = Path(__file__).resolve().parent.parent / "docs" / "tasks" / "greyed-items.md"
LINE = re.compile(r"^- `([A-Za-z0-9]+)` (.+)$")
WINDOWS = re.compile(r"\s*\[[^\]]*\]\s*$")


def tag_key(tag):
    """Briefs first (T58), then backlog packages (B4), then N, each in number order."""
    m = re.match(r"([A-Za-z]+)(\d*)$", tag)
    letter, number = m.group(1), int(m.group(2) or 0)
    return ("TBN".find(letter[0]) % 4, number, tag)


def main():
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT
    part = None
    parts = {"Menu bar": 0, "Toolbars": 0, "Context menus": 0}
    packages = {}
    untagged = []
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if raw.startswith("## "):
            part = raw[3:].strip() if raw[3:].strip() in parts else None
            continue
        if part is None or not raw.startswith("- "):
            continue
        m = LINE.match(raw)
        if not m:
            untagged.append(number)
            continue
        tag, body = m.group(1), WINDOWS.sub("", m.group(2))
        if ": " in body:
            body = body.split(": ", 1)[1]
        count = len([item for item in body.split(" · ") if item.strip()])
        parts[part] += count
        packages[tag] = packages.get(tag, 0) + count

    total = sum(parts.values())
    print(f"{total} greyed items: {parts['Menu bar']} in the menu bar, "
          f"{parts['Toolbars']} on the toolbars, {parts['Context menus']} in context menus.")
    print()
    print("| Package | Items |")
    print("|---|---|")
    for tag in sorted(packages, key=tag_key):
        print(f"| `{tag}` | {packages[tag]} |")
    if untagged:
        print(f"\nLines without a package tag: {', '.join(map(str, untagged))}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

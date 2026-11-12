"""Recounts the Counts table of docs/spec/feature-parity.md from its rows.

Usage: python tools/parity-count.py [path]   (default: docs/spec/feature-parity.md)
Each table row has the columns Feature | Origin | Sheaf | Status | Gap | Priority, with the
status as one of the marks below and the priority as P0, P1 or P2.
"""
import re
import sys

MARKS = ("✓", "◐", "✗")
SKIP = ("| Feature", "|---", "| |", "| All rows", "| P0 |", "| P1 and P2")


def main() -> None:
    path = sys.argv[1] if len(sys.argv) > 1 else "docs/spec/feature-parity.md"
    text = open(path, encoding="utf-8").read()
    counts: dict[tuple[str, str], int] = {}
    for line in text.split("\n"):
        if not line.startswith("| ") or line.startswith(SKIP):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 6 or cells[3] not in MARKS or cells[5] not in ("P0", "P1", "P2"):
            continue
        key = (cells[5], cells[3])
        counts[key] = counts.get(key, 0) + 1

    def row(name: str, keep) -> str:
        v = [sum(n for (p, m), n in counts.items() if m == mark and keep(p)) for mark in MARKS]
        return f"| {name} | {v[0]} | {v[1]} | {v[2]} | {sum(v)} |"

    text = re.sub(r"\| All rows \|[^\n]*", row("All rows", lambda p: True), text, count=1)
    text = re.sub(r"\| P0 \| \d+ \| \d+ \| \d+ \| \d+ \|", row("P0", lambda p: p == "P0"), text, count=1)
    text = re.sub(r"\| P1 and P2 together \|[^\n]*", row("P1 and P2 together", lambda p: p != "P0"), text, count=1)
    open(path, "w", encoding="utf-8").write(text)
    print(row("All rows", lambda p: True))
    print(row("P0", lambda p: p == "P0"))


if __name__ == "__main__":
    main()

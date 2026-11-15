"""Count the hand-maintained greyed-item inventory, not the Rust source.

Run from any directory: python3 tools/greyed-count.py [--check | --write]
--check fails if generated counts are stale. --write refreshes inventory, plan and handoff counts.
One label per inventory location is counted; [window] qualifiers do not multiply the count.
"""
from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
SECTIONS = ("Menu bar", "Toolbars", "Context menus")
PACKAGES = tuple(f"T{i}" for i in range(56, 64)) + tuple(f"B{i}" for i in range(1, 17)) + ("N",)
START = "<!-- greyed-count:packages:start -->"
END = "<!-- greyed-count:packages:end -->"


def count_items(source: str, allowed_packages=PACKAGES) -> tuple[Counter, Counter]:
    packages: Counter = Counter({p: 0 for p in allowed_packages})
    sections: Counter = Counter()
    section = None
    seen = set()
    for number, line in enumerate(source.splitlines(), 1):
        if line.startswith("## "):
            section = line[3:].strip()
            if section in SECTIONS:
                seen.add(section)
        if section not in SECTIONS or not line.startswith("- "):
            continue
        match = re.fullmatch(r"- `([^`]+)` (.+)", line)
        if not match or match[1] not in allowed_packages:
            raise ValueError(f"line {number}: missing or unknown package tag")
        labels = match[2].split(" · ")
        if any(not label.strip() for label in labels):
            raise ValueError(f"line {number}: empty item between separators")
        packages[match[1]] += len(labels)
        sections[section] += len(labels)
    if seen != set(SECTIONS):
        raise ValueError("inventory must contain all three surface sections")
    return packages, sections


def refreshed(files: dict[str, str]) -> tuple[dict[str, str], Counter, Counter]:
    inventory = files["docs/tasks/greyed-items.md"]
    plan = files["docs/tasks/PLAN.md"]
    plan_packages = []
    for line in plan.splitlines():
        if re.match(r"\| \[[ x]\] \|", line):
            cells = [c.strip() for c in line.strip("|").split("|")]
            match = re.fullmatch(r"(?:\[(T[1-9]\d*)\]\([^)]*\)|(B[1-9]\d*))", cells[1])
            if not match:
                raise ValueError(f"unrecognized plan package row: {line}")
            package = match[1] or match[2]
            if package in plan_packages:
                raise ValueError(f"unknown/duplicate plan package {package}")
            plan_packages.append(package)
    if not plan_packages:
        raise ValueError("plan has no package rows")
    package_order = sorted(plan_packages, key=lambda p: (p[0] != "T", int(p[1:]))) + ["N"]
    packages, sections = count_items(inventory, package_order)
    total = sum(packages.values())
    surface = f"{sections['Menu bar']} in the menu bar, {sections['Toolbars']} on the toolbars, {sections['Context menus']} in context menus"
    inventory, n = re.subn(r"\*\*\d+ greyed items:.*?\*\*", f"**{total} greyed items: {surface}.**", inventory, count=1, flags=re.S)
    if n != 1 or inventory.count(START) != 1 or inventory.count(END) != 1:
        raise ValueError("inventory count header or generated table markers missing/duplicated")
    table = "| Package | Items |\n|---|---|\n" + "\n".join(f"| `{p}` | {packages[p]} |" for p in package_order)
    inventory = re.sub(re.escape(START) + r".*?" + re.escape(END), START + "\n" + table + "\n" + END, inventory, flags=re.S)
    inventory, n = re.subn(r"Briefs \(`T`\) hold \d+ items, backlog packages \(`B`\) \d+, not planned \(`N`\) \d+\.",
        f"Briefs (`T`) hold {sum(packages[p] for p in package_order if p.startswith('T'))} items, backlog packages (`B`) {sum(packages[p] for p in package_order if p.startswith('B'))}, not planned (`N`) {packages['N']}.", inventory)
    if n != 1:
        raise ValueError("inventory package summary missing/duplicated")
    plan = re.sub(r"the \d+ greyed items", f"the {total} greyed items", plan)
    plan = re.sub(r"## Not planned \(`N`, \d+ items\)", f"## Not planned (`N`, {packages['N']} items)", plan)
    rows = []
    found = set()
    for line in plan.splitlines():
        if re.match(r"\| \[[ x]\] \|", line):
            cells = [c.strip() for c in line.strip("|").split("|")]
            match = re.fullmatch(r"(?:\[(T\d+)\]\([^)]*\)|(B\d+))", cells[1])
            if not match:
                raise ValueError(f"unrecognized plan package row: {line}")
            package = match[1] or match[2]
            if package not in package_order or package in found:
                raise ValueError(f"unknown/duplicate plan package {package}")
            found.add(package)
            if len(cells) not in (5, 7):
                raise ValueError(f"unexpected plan table shape for {package}")
            cells[5 if len(cells) == 7 else 4] = str(packages[package])
            line = "| " + " | ".join(cells) + " |"
        rows.append(line)
    if found != set(package_order) - {"N"}:
        raise ValueError("plan must contain each T/B package exactly once")
    handoff, n = re.subn(r"\*\*\d+ listed item locations\*\* at the planning\n  baseline \(\d+ menu bar, \d+ toolbar, \d+ context menu\)",
        f"**{total} listed item locations** at the planning\n  baseline ({sections['Menu bar']} menu bar, {sections['Toolbars']} toolbar, {sections['Context menus']} context menu)", files["HANDOFF.md"])
    if n != 1:
        raise ValueError("handoff inventory summary missing/duplicated")
    return {"docs/tasks/greyed-items.md": inventory, "docs/tasks/PLAN.md": "\n".join(rows) + "\n", "HANDOFF.md": handoff}, packages, sections


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write", action="store_true")
    args = parser.parse_args()
    paths = ("docs/tasks/greyed-items.md", "docs/tasks/PLAN.md", "HANDOFF.md")
    try:
        original = {p: (ROOT / p).read_text(encoding="utf-8") for p in paths}
        updated, packages, sections = refreshed(original)
    except (OSError, ValueError) as error:
        parser.exit(2, f"greyed-count: {error}\n")
    stale = [p for p in paths if updated[p] != original[p]]
    print(f"Total: {sum(packages.values())}; " + "; ".join(f"{s}: {sections[s]}" for s in SECTIONS))
    print("; ".join(f"{p}: {packages[p]}" for p in packages))
    if args.write:
        for p in stale:
            (ROOT / p).write_text(updated[p], encoding="utf-8")
        print("Updated: " + (", ".join(stale) or "none"))
    elif args.check and stale:
        print("Stale counts: " + ", ".join(stale) + "; run with --write")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

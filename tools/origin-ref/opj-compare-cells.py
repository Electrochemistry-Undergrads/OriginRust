#!/usr/bin/env python3
"""Compares two cell listings of an Origin project: the one Origin wrote
(opj-dump-cells.ps1) and the one our reader wrote
(`cargo run -p sheaf-io --example opj_dump -- FILE.opj --cells`).

Usage: opj-compare-cells.py ORIGIN_LISTING OUR_LISTING [--details FILE] [--max-show N]

The format is described in crates/io/examples/opj_dump.rs. Workbooks are matched by window
name, sheets by position inside the workbook, columns by position inside the sheet. A column
counts as matching when its short name, long name, units, comments, designation and used-row
count are all equal; every one of these that differs is counted and listed separately.
A cell matches when both sides have the same kind (number, text, missing) and the same
value: numbers must be the same double, bit for bit; text must be identical.
One special case is counted apart: our reader turns a column that holds both text and numbers
into a text column, writing the numbers as text. When Origin says number and we say text, and
the text reads as exactly that double, the cell is counted as "number kept as text", not as a
mismatch. The program prints counts and the first differences of each kind; with --details it
writes every difference to a file. Exit status 0 when nothing differs, 1 otherwise.
"""
import sys
import argparse
from collections import Counter, defaultdict


def parse(path):
    """Returns {window name: (long name, [sheet])} in file order as a list of books."""
    books = []
    book = sheet = col = None
    notes = []
    errors = []
    with open(path, encoding="utf-8", newline="") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            parts = line.split("\t")
            kind = parts[0]
            if kind == "BOOK":
                book = {"name": parts[1], "long": parts[2] if len(parts) > 2 else "", "sheets": []}
                books.append(book)
            elif kind == "SHEET":
                sheet = {"name": parts[1], "cols": []}
                book["sheets"].append(sheet)
            elif kind == "COL":
                col = {
                    "index": int(parts[1]),
                    "short": parts[2],
                    "long": parts[3],
                    "units": parts[4],
                    "comments": parts[5],
                    "role": parts[6],
                    "used": int(parts[7]),
                    "cells": {},
                }
                sheet["cols"].append(col)
            elif kind == "CELL":
                value = parts[4] if len(parts) > 4 else ""
                col["cells"][int(parts[2])] = (parts[3], value)
            elif kind == "ERR":
                errors.append(line)
            elif kind.startswith("#"):
                notes.append(line)
    return books, notes, errors


def as_float(text):
    try:
        return float(text)
    except ValueError:
        return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("origin")
    ap.add_argument("ours")
    ap.add_argument("--details")
    ap.add_argument("--max-show", type=int, default=8)
    a = ap.parse_args()

    ob, onotes, oerr = parse(a.origin)
    ub, unotes, uerr = parse(a.ours)
    counts = Counter()
    diffs = defaultdict(list)

    def diff(category, text):
        diffs[category].append(text)

    for e in oerr:
        diff("origin could not read", e)
    ours_by_name = {}
    for b in ub:
        ours_by_name.setdefault(b["name"], []).append(b)
    used_ours = set()
    for b in ob:
        counts["books in Origin"] += 1
        cands = [c for c in ours_by_name.get(b["name"], []) if id(c) not in used_ours]
        if not cands:
            diff("workbook missing in our reader", f"{b['name']!r} ({len(b['sheets'])} sheets)")
            continue
        u = cands[0]
        used_ours.add(id(u))
        counts["books matched by name"] += 1
        if b["long"] != u["long"]:
            diff("workbook long name differs", f"{b['name']!r}: Origin {b['long']!r} ours {u['long']!r}")
        if len(b["sheets"]) != len(u["sheets"]):
            diff("sheet count differs", f"{b['name']!r}: Origin {len(b['sheets'])} ours {len(u['sheets'])}")
        for si, (bs, us) in enumerate(zip(b["sheets"], u["sheets"])):
            counts["sheets compared"] += 1
            where = f"{b['name']!r}/{bs['name']!r}"
            if bs["name"] != us["name"]:
                diff("sheet name differs", f"{b['name']!r} sheet {si}: Origin {bs['name']!r} ours {us['name']!r}")
            if len(bs["cols"]) != len(us["cols"]):
                diff("column count differs", f"{where}: Origin {len(bs['cols'])} ours {len(us['cols'])}")
            for bc, uc in zip(bs["cols"], us["cols"]):
                counts["columns compared"] += 1
                cw = f"{where} col {bc['index']} ({bc['short']!r})"
                col_ok = True
                for key, label in (
                    ("short", "short name"),
                    ("long", "long name"),
                    ("units", "units"),
                    ("comments", "comments"),
                    ("role", "designation"),
                    ("used", "used rows"),
                ):
                    counts[f"column {label} compared"] += 1
                    if bc[key] != uc[key]:
                        col_ok = False
                        diff(f"column {label} differs", f"{cw}: Origin {bc[key]!r} ours {uc[key]!r}")
                    else:
                        counts[f"column {label} matched"] += 1
                if col_ok:
                    counts["columns fully matching"] += 1
                for row, (ok_kind, ov) in sorted(bc["cells"].items()):
                    counts["cells compared"] += 1
                    uk = uc["cells"].get(row)
                    if uk is None:
                        diff("cell not listed by our reader", f"{cw} row {row}: Origin {ok_kind} {ov!r}")
                        continue
                    uk_kind, uv = uk
                    if ok_kind == "M" and uk_kind == "M":
                        counts["cells matched"] += 1
                    elif ok_kind == "N" and uk_kind == "N":
                        x, y = as_float(ov), as_float(uv)
                        if x is not None and y is not None and (x == y or (x != x and y != y)) and repr(x) == repr(y):
                            counts["cells matched"] += 1
                        else:
                            diff("number differs", f"{cw} row {row}: Origin {ov} ours {uv}")
                    elif ok_kind == "T" and uk_kind == "T":
                        if ov == uv:
                            counts["cells matched"] += 1
                        else:
                            diff("text differs", f"{cw} row {row}: Origin {ov!r} ours {uv!r}")
                    elif ok_kind == "N" and uk_kind == "T" and as_float(uv) == as_float(ov):
                        counts["cells matched"] += 1
                        counts["of which number kept as text"] += 1
                    else:
                        diff("kind differs", f"{cw} row {row}: Origin {ok_kind} {ov!r} ours {uk_kind} {uv!r}")
    for b in ub:
        if id(b) not in used_ours:
            diff("workbook only in our reader", f"{b['name']!r} ({len(b['sheets'])} sheets)")
    counts["books in our reader"] = len(ub)
    counts["columns in Origin"] = sum(len(s["cols"]) for b in ob for s in b["sheets"])
    counts["columns in our reader"] = sum(len(s["cols"]) for b in ub for s in b["sheets"])

    print("== counts")
    for k, v in counts.items():
        print(f"  {k}: {v}")
    print("== differences")
    if not diffs:
        print("  none")
    for cat, items in diffs.items():
        print(f"  {cat}: {len(items)}")
        for t in items[: a.max_show]:
            print(f"      {t}")
    if a.details:
        with open(a.details, "w", encoding="utf-8") as f:
            for cat, items in diffs.items():
                for t in items:
                    f.write(f"{cat}\t{t}\n")
    sys.exit(1 if diffs else 0)


if __name__ == "__main__":
    main()

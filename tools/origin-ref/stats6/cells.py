"""Prints the non-empty cells of a '$wcol(c)[r]$' dump (rows as lines). usage: python cells.py RAW [c0 c1] [r0 r1]"""
import re, sys
cells = {}
for l in open(sys.argv[1], encoding='utf-8', errors='replace'):
    m = re.match(r'wcol\((\d+)\)\[(\d+)\]\$ = (.*)', l.rstrip('\r\n'))
    if m and m.group(3).strip(): cells[(int(m.group(1)), int(m.group(2)))] = m.group(3)
c0, c1 = (int(sys.argv[2]), int(sys.argv[3])) if len(sys.argv) > 3 else (1, 9999)
r0, r1 = (int(sys.argv[4]), int(sys.argv[5])) if len(sys.argv) > 5 else (1, 9999)
for r in sorted({r for c, r in cells if r0 <= r <= r1}):
    print(r, [(c, cells[(c, r)]) for c in sorted({c for c, rr in cells if rr == r and c0 <= c <= c1})])

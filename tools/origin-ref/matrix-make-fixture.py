"""Turns the printed output of matrix-ops.lines.txt and matrix-conv.lines.txt (run with
run-labtalk-lines.ps1) into crates/core/tests/data/origin_matrix.json.

Usage: python matrix-make-fixture.py OPS_OUTPUT CONV_OUTPUT OUT_JSON

Each case becomes {"rows": [[...]], "x": [...], "y": [...]} (a missing value is null), plus
"ok": false when Origin did not make a matrix (the active window stayed a workbook), and
"stats" for the mstats case. For worksheets (m2w) the trailing rows that are all missing (the
empty rows of a new worksheet) are dropped.
"""

import json
import re
import sys


def num(tok):
    return None if tok == '--' else float(tok)


def parse(path):
    cases = {}
    cur = None
    for line in open(path, encoding='utf-8', errors='replace'):
        line = line.rstrip('\n')
        m = re.match(r'^(\w+)\$ = (.*)$', line)
        if line.startswith('['):
            if cur is not None:
                cur['_last'] = None
            continue
        if not m:
            # a continuation of the previous string (Origin wraps long strings)
            if cur is not None and cur.get('_last') and line and not line.startswith('['):
                cur[cur['_last']] += ' ' + line.strip()
            continue
        key, val = m.group(1), m.group(2)
        if key == 'cn':
            cur = {'_last': None}
            cases[val.replace('CASE ', '').strip()] = cur
            continue
        if cur is None:
            continue
        cur[key] = val
        cur['_last'] = key
    return cases


def rows_of(d):
    rows = []
    for part in d.split('/'):
        toks = part.split()
        if toks:
            rows.append([num(t) for t in toks])
    return rows


def convert(cases, worksheet_prefix='m2w'):
    out = {}
    for name, c in cases.items():
        e = {}
        if 'pn' in c:
            e['ok'] = c['pn'].startswith('MBook')
        if 'd' in c and e.get('ok', True) and 'dx' in c:
            e['rows'] = rows_of(c['d'])
            e['x'] = [num(t) for t in c['dx'].split()]
            e['y'] = [num(t) for t in c['dy'].split()]
        elif 'd' in c and name.startswith(worksheet_prefix):
            rows = rows_of(c['d'])
            while rows and all(v is None for v in rows[-1]):
                rows.pop()
            e['rows'] = rows
        if 'st' in c:
            t = c['st'].split()
            e['stats'] = dict(zip(['mean', 'sd', 'n', 'min', 'max', 'sum', 'missing'], [float(v) for v in t[1:]]))
        out[name] = e
    return out


ops = convert(parse(sys.argv[1]))
conv = convert(parse(sys.argv[2]))
for k in list(ops):
    if k.startswith('m2w'):
        del ops[k]
ops.update(conv)
json.dump(ops, open(sys.argv[3], 'w'), indent=1, sort_keys=True)
print(len(ops), 'cases')

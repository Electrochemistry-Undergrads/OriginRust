"""Parses the raw output of a g1.py run: {case: dict(page, sheet, ncols, cols={k: dict(lname, unit, comment, type,
format)}, cells={(c, r): text})}; empty cells are left out."""
import re, sys

def parse(path):
    res = {}; cur = None; dump = None; idx = 0
    for line in open(path, encoding='utf-8', errors='replace').read().split('\n'):
        line = line.rstrip('\r')
        m = re.match(r'#(?:NOARRAY )?CASE (\d+) (\S+)', line)
        if m:
            cur = dict(name=m.group(2), cols={}, cells={}); res[m.group(2)] = cur; dump = None; continue
        if cur is None:
            continue
        m = re.match(r'(page\.name|wks\.name)\$ = (.*)$', line)
        if m:
            cur['page' if m.group(1) == 'page.name' else 'sheet'] = m.group(2); continue
        m = re.match(r'wks\.ncols = (\S+)', line)
        if m:
            cur['ncols'] = int(float(m.group(1))); continue
        m = re.match(r'wks\.col(\d+)\.(lname|unit|comment)\$ = (.*)$', line)
        if m:
            cur['cols'].setdefault(int(m.group(1)), {})[m.group(2)] = m.group(3); continue
        m = re.match(r'wks\.col(\d+)\.(type|format) = (\S+)', line)
        if m:
            cur['cols'].setdefault(int(m.group(1)), {})[m.group(2)] = m.group(3); continue
        m = re.match(r'#(?:NOARRAY|SHEET|ERR) DUMP (\S+) (\d+) (\d+)', line)
        if m:
            dump = int(m.group(3)); idx = 0; continue
        if dump is None:
            continue
        m = re.match(r'ds\$ = (.*)$', line)
        if m:
            c, r = idx // dump + 1, idx % dump + 1; idx += 1
            if m.group(1) != '':
                cur['cells'][(c, r)] = m.group(1)
    return res

if __name__ == '__main__':
    import json
    cases = {c['name']: c for c in json.load(open(sys.argv[2]))}
    for n, c in parse(sys.argv[1]).items():
        nc = c.get('ncols', 0)
        print('==', n, cases[n]['opts'], c.get('page'), c.get('sheet'), 'ncols', nc)
        for k in range(1, nc + 1):
            print('   col', k, c['cols'].get(k))
        rows = max([r for (_, r) in c['cells']] or [0])
        for r in range(1, rows + 1):
            print('    ', [c['cells'].get((k, r), '') for k in range(1, nc + 1)])

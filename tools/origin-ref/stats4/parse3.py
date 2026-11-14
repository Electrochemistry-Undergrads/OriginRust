"""Parses the raw output of a g3.py run: {case_name: dict(ncols, nrows, lname=[...], comment=[...], cells={(c, r): value})};
empty cells are missing from `cells`; `ok` is False when the X-Function reported an error."""
import re, sys

def parse(path):
    res = {}; cur = None; dump = None; pend = None; ok = True; idx = 0
    for line in open(path, encoding='utf-8', errors='replace').read().split('\n'):
        line = line.rstrip('\r')
        m = re.match(r'\[(True|False)\] twoDBinning', line)
        if m:
            ok = m.group(1) == 'True'; continue
        m = re.match(r'#(?:NOARRAY )?CASE (\d+) (\S+)', line)
        if m:
            cur = dict(index=int(m.group(1)), name=m.group(2), ok=ok, lname={}, comment={}, unit={}, cells={}); res[m.group(2)] = cur; dump = None; continue
        if cur is None:
            continue
        m = re.match(r'wks\.(ncols|nrows) = (\S+)', line)
        if m:
            cur[m.group(1)] = float(m.group(2)); continue
        m = re.match(r'wks\.col(\d+)\.(lname|comment|unit)\$ = (.*)$', line)
        if m:
            cur[m.group(2)][int(m.group(1))] = m.group(3); continue
        m = re.match(r'#(?:NOARRAY|SHEET) DUMP (\S*) (\S+) (\d+)', line)
        if m:
            dump = int(m.group(3)); idx = 0; continue
        if dump is None:
            continue
        m = re.match(r'dv = (.*)$', line)
        if m:
            pend = m.group(1); continue
        m = re.match(r'ds\$ = (.*)$', line)
        if m:
            c, r = idx // dump + 1, idx % dump + 1; idx += 1
            s = m.group(1)
            if s != '':
                try:
                    float(s); cur['cells'][(c, r)] = float(pend)
                except ValueError:
                    cur['cells'][(c, r)] = s
    return res

if __name__ == '__main__':
    for n, c in parse(sys.argv[1]).items():
        print(n, c.get('ok'), c.get('ncols'), c.get('nrows'), [c['lname'].get(k) for k in range(1, 5)], [c['comment'].get(k) for k in range(1, 5)])

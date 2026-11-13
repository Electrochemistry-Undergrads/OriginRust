"""Parses the raw output of run-labtalk-safe.ps1 for the Holm runs: returns {(case_index, method): (rc, rows)}
where rows is the list of dumped sheet rows (each a list of cell strings)."""
import re, sys

def parse(path):
    res = {}; cur = None; last = None
    for line in open(path, encoding='utf-8', errors='replace').read().split('\n'):
        line = line.rstrip('\r')
        m = re.match(r'r(\d+)_(\d+) = (\S+)$', line)
        if m:
            last = (int(m.group(1)), int(m.group(2))); res[last] = [float(m.group(3)), []]; cur = None; continue
        if line.startswith('#SHEET'):
            cur = last; continue
        if line.startswith('[True]') or line.startswith('[False]') or line.startswith('#'):
            cur = None; continue
        if cur is not None and line != '' or (cur is not None and '\t' in line):
            res[cur][1].append(line.split('\t'))
    return res

if __name__ == '__main__':
    r = parse(sys.argv[1])
    for k in sorted(r):
        print(k, r[k][0], len(r[k][1]))

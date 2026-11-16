"""Reads the raw output of a stats6 line script run (dump2 format) into per-dump cell dictionaries.
usage: python pp.py RAW.txt  (prints the non-empty cells per dump, rows as lines)"""
import re, sys


def parse(path):
    dumps = []; cur = None; pend = None
    for line in open(path, encoding='utf-8', errors='replace').read().split('\n'):
        line = line.rstrip('\r')
        m = re.match(r'#(?:NOARRAY|SHEET) DUMP (\S+) (\S+) (\d+)', line)
        if m:
            cur = dict(tag=m.group(1), nr=int(m.group(3)), cells={}, num={}, n=0); dumps.append(cur); continue
        if cur is None: continue
        m = re.match(r'^dv = (.*)$', line)
        if m: pend = m.group(1); continue
        m = re.match(r'^ds\$ = (.*)$', line)
        if m:
            i = cur['n']; cur['n'] += 1
            has_num = pend is not None
            # numeric dumps have dv then ds per cell; text dumps only ds
            k = (i // cur['nr'] + 1, i % cur['nr'] + 1)
            s = m.group(1)
            if s != '': cur['cells'][k] = s
            if has_num and pend != '-1.23456789E-300': cur['num'][k] = pend
            pend = None
    return dumps


def show(path, tags=None):
    for d in parse(path):
        if tags and d['tag'] not in tags: continue
        print('==', d['tag'])
        cs = sorted({c for c, r in d['cells']}); rs = sorted({r for c, r in d['cells']})
        for r in rs:
            print(r, [d['cells'].get((c, r), '') for c in cs])


if __name__ == '__main__':
    show(sys.argv[1], sys.argv[2:] or None)

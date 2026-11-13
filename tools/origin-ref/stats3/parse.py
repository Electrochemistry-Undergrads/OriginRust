import re,sys
def parse(path):
    out=[]; cur=None; pend=None
    for line in open(path,encoding='utf-8',errors='replace').read().split('\n'):
        line=line.rstrip('\r')
        m=re.match(r'#(?:NOARRAY|SHEET) VARS (\S+)',line)
        if m: out.append(('mark',m.group(1))); continue
        m=re.match(r'#(?:NOARRAY|SHEET) VARS (\S+)',line)
        if m: out.append(('mark',m.group(1))); continue
        m=re.match(r'#(?:NOARRAY|SHEET) DUMP (\S*) (\S+) (\d+)',line)
        if m: cur={'nr':int(m.group(3)),'tag':m.group(1),'sheet':m.group(2),'cells':{}}; out.append(cur); continue
        m=re.match(r'^dv = (.*)$',line)
        if m and cur is not None: pend=m.group(1); continue
        m=re.match(r'^ds\$ = (.*)$',line)
        if m and cur is not None:
            i=len(cur['cells']); k=(i//cur['nr']+1,i%cur['nr']+1)
            s=m.group(1)
            try:
                float(s); v=pend
            except: v=s
            cur['cells'][k]=v if s!='' else None; continue
        m=re.match(r'^(\w[\w.$]*) = (.*)$',line)
        if m: out.append(('var',m.group(1),m.group(2)))
    return out
def show(path):
    for d in parse(path):
        if isinstance(d,tuple): print('VAR',d[1],d[2]); continue
        g={k:v for k,v in d['cells'].items() if v is not None}
        print('SHEET',d['tag'],d['sheet'])
        cs=sorted({c for c,r in g}); rs=sorted({r for c,r in g})
        for r in rs:
            print('  r%d: '%r+' | '.join(f"c{c}={g[(c,r)]}" for c in cs if (c,r) in g))
if __name__=='__main__' and sys.argv[1]!='compact': show(sys.argv[1])


def compact(path, out):
    lines = []
    cur = None
    for d in parse(path):
        if isinstance(d, tuple):
            if d[0] == 'mark':
                cur = d[1]; lines.append('#VARS ' + cur)
            elif d[0] == 'var' and cur is not None and d[1] in ('a', 'b', 'c', 'd', 'e', 'f', 'g$'):
                lines.append(f'{d[1]}={d[2]}')
            continue
        cur = None
        lines.append('#SHEET ' + d['tag'])
        for (c, r), v in sorted(d['cells'].items()):
            if v is not None:
                lines.append(f'{c} {r} {v}')
    open(out, 'w').write('\n'.join(lines) + '\n')


if __name__ == '__main__' and len(sys.argv) > 3 and sys.argv[1] == 'compact':
    compact(sys.argv[2], sys.argv[3])

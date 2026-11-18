import math, subprocess, re
import os
RUN=os.path.join(os.path.dirname(os.path.abspath(__file__)),'..','run-labtalk-safe.ps1')
def sig(n,a=0.5,b=0.1): return [round(math.sin(a*i)+b*i+0.3*math.cos(2.1*i*i/7),6) for i in range(n)]
def book(L,name,cols):
    L.append(f'newbook name:="{name}" sheet:=1;')
    for i,c in enumerate(cols):
        if i>1: L.append("wks.addcol();")
        L.append(f"col({i+1}) = {{{', '.join(repr(float(v)) for v in c)}}};")
def dump(L,name,tag=None):
    L.append(f"$\"@@{name}\"")  # marker unsupported; use string var
def run(L,fn,timeout=900):
    open(fn+'.lines.txt','w').write('\n'.join(L)+'\n')
    out=subprocess.run(['powershell','-File',RUN,'-Script',fn+'.lines.txt','-TimeoutSec','240'],capture_output=True,text=True,timeout=timeout).stdout
    open(fn+'.out','w').write(out); return out
def parse(out):
    sheets={}; cur=None
    for ln in out.splitlines():
        m=re.match(r'#SHEET (\S+) (\d+) (\d+)',ln)
        if m: cur=m.group(1); sheets[cur]=[]; continue
        if cur and (ln.startswith('wks.') or ln.startswith('$')): cur=None
        elif cur and not ln.startswith('['): sheets[cur].append(ln.split('\t'))
        elif ln.startswith('['): cur=None
    # transposed to columns
    res={}
    for k,rows in sheets.items():
        nc=max(len(r) for r in rows) if rows else 0
        res[k]=[[float(r[c]) for r in rows if c<len(r) and r[c]!=''] for c in range(nc)]
    return res
def stat(out): return [l for l in out.splitlines() if l.startswith('[False]') or 'ERR' in l or 'TIMEOUT' in l]

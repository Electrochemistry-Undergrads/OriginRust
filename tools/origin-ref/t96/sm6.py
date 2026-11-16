from lib import *; from n1 import noisy; from wt import *
from wtm import *

L=[]; cases=[]
def add(n,ty,ext,c): cases.append((n,ty,ext,c))
for c in (1,2,3): add(128,'db2','periodic',c)
for c in (0,1,50): add(128,'haar','periodic',c)
for c in (0,1,2,50): add(128,'db4','periodic',c)
for c in (0,50): add(64,'db2','periodic',c)
for c in (0,50): add(100,'db2','periodic',c)
for c in (0,50,100): add(100,'db2','pad0',c)
for c in (0,50,100): add(128,'db2','pad0',c)
for c in (0,50): add(128,'b13','periodic',c)
for i,(n,ty,ext,c) in enumerate(cases):
    y=noisy(n); book(L,f'Z{i}',[list(range(n)),y]); L.append(f"wtsmooth iy:=(1,2) type:={ty} ext:={ext} cutoff:={c};"); L.append(f"#[Z{i}]Sheet1")
open('sm6.cases','w').write(repr(cases))
out=run(L,'sm6'); print(stat(out))

from lib import *
from n1 import noisy
L=[]; x=list(range(128)); y=noisy(128)
cases=[]
def add(**k): cases.append(k)
for lv in (1,2,3,4): 
    for thrd in (0,25,50,100): add(type='db2',ext='periodic',thrt='custom',level=lv,thrd=thrd)
for lv in (1,2,3,5): add(type='db2',ext='periodic',thrt='sqtwolog',level=lv,thrd=50)
add(type='db2',ext='periodic',thrt='sqtwolog',level=2,thrd=10)
for t in ('haar','db4','b13','b37'): add(type=t,ext='periodic',thrt='custom',level=2,thrd=50)
for t in ('haar','db4'): add(type=t,ext='pad0',thrt='custom',level=2,thrd=50)
add(type='db4',ext='pad0',thrt='sqtwolog',level=2,thrd=50)
add(type='db2',ext='periodic',thrt='custom',level=7,thrd=50)
add(type='db2',ext='periodic',thrt='custom',level=6,thrd=50)
for i,c in enumerate(cases):
    book(L,f'W{i}',[x,y]);
    L.append("wtdenoise iy:=(1,2) type:={type} ext:={ext} thrt:={thrt} level:={level} thrd:={thrd};".format(**c))
    L.append(f"#[W{i}]Sheet1"); L.append("$wks.col5.comment$")
open('dn.cases','w').write(repr(cases))
out=run(L,'dn1'); print(stat(out))

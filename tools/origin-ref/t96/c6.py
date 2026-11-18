from lib import *
L=[]
def imp(n,m,v=1.0): x=[0.0]*n; x[m]=v; return x
cases=[]
def add(nm,x,s,ty,par):
    cases.append((nm,ty,par,len(x),s)); book(L,nm,[x,s])
    L.append(f"cwt ix1:=col(1) ix2:=col(2) type:={ty} param:={par} upf:=0 plot:=0;")
    L.append(f"$wks.name$"); L.append("?wks.nCols"); L.append("?wks.nRows")
    L.append(f"#[{nm}]CWTResults1")
    L.append(f"$wks.col1.name$")
sc=[2.0,3.0,5.0]
for p in range(1,13):
    add('P%d'%p,imp(64,32),sc,1,p)
for p in (1,2,3,4,5,6,7,8,9,10,15,20):
    add('Q%d'%p,imp(64,32),sc,0,p)
out=run(L,'cwt6'); print(stat(out))
open('cwt6.cases','w').write(repr(cases))

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
sc=[2.0,3.0]
for p in (21,25,50,100,5.5,6.5,19.9):
    add('M%s'%str(p).replace('.','_'),imp(64,32),sc,0,p)
for p in (0,8.5,1.5,2.5):
    add('D%s'%str(p).replace('.','_'),imp(64,32),sc,1,p)
add('S2',[1.0,2.0],[1.0,2.0],0,5)
add('S3',[1.0,2.0,0.5],[1.0],2,5)
add('S1',[1.0],[1.0],2,5)
add('N0',imp(64,32),[3.0,2.0,1.0,4.0],2,5)
out=run(L,'cwt7'); print(stat(out))
open('cwt7.cases','w').write(repr(cases))

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
sc=[1.2,1.7,2.5,2.9,3.5,5.5,7.9,12.3]
add('F1',imp(64,32),sc,0,5)
add('F2',imp(64,32),sc,2,5)
add('F3',imp(64,32),sc,1,3)
add('H1',imp(64,32),[0.9,2.0],0,5)
add('H2',imp(64,32),[2.0,0.5],0,5)
add('H3',imp(64,32),[2.0,0.99999,1.0],0,5)
sg=sig(65)
add('G1',sg,[1.0,2.0,3.0,5.0],0,5)
add('G2',sg[:64],[1.0,2.0,3.0,5.0],2,5)
add('G3',sig(30),[2.0,4.0,16.0,40.0],0,6)
add('G4',sig(31),[2.0,4.0,16.0,40.0],1,4)
add('G5',sig(17),[1.0,2.0,3.0,40.0],2,5)
out=run(L,'cwt5'); print(stat(out))
open('cwt5.cases','w').write(repr(cases))

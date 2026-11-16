from lib import *
L=[]; n=64
def imp(m,v=1.0): x=[0.0]*n; x[m]=v; return x
sc=[1.5,2.0,3.0,4.0,6.0,8.0]
cases=[]
def add(nm,x,s,ty,par,upf=0,plot=0):
    cases.append((nm,ty,par,upf)); book(L,nm,[x,s])
    L.append(f"cwt ix1:=col(1) ix2:=col(2) type:={ty} param:={par} upf:={upf} plot:={plot};")
    L.append(f"$wks.name$"); L.append("?wks.nCols"); L.append("?wks.nRows"); L.append("?page.nLayers")
    L.append(f"#[{nm}]CWTResults1")
    for c in range(1,len(s)+3): L.append(f"$wks.col{c}.name$"); L.append(f"$wks.col{c}.lname$"); L.append(f"$wks.col{c}.unit$")
add('CM',imp(32),sc,0,5); add('CD',imp(32),sc,1,5); add('CX',imp(32),sc,2,5)
add('CM3',imp(32),sc,0,3); add('CM8',imp(32),sc,0,8); add('CD2',imp(32),sc,1,2)
add('CMS',imp(20,2.0),sc,0,5)
add('CMP',imp(32),sc,0,5,upf=1)
add('CMI',imp(32),sc,0,5,plot=1)
out=run(L,'cwt2'); print(stat(out))
open('cwt2.cases','w').write(repr(cases))

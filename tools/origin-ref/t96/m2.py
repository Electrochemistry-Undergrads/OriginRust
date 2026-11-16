from lib import *
L=[]
x16=sig(16); x21=sig(21); x64=sig(64,0.3,0.02)
cases=[('M1',x16,'db2',0,2),('M2',x16,'db2',1,2),('M3',x21,'db3',1,3),('M4',x64,'haar',0,3),('M5',x64,'db4',1,4),('M8',x16,'haar',0,3),('M9',x16,'haar',1,4)]
for (nm,x,ty,ext,lv) in cases:
    book(L,nm,[x]); L.append(f"mdwt ix:=col(1) type:={ty} ext:={ext} level:={lv};"); L.append(f"#[{nm}]MDWT1")
    for c in range(1,3+lv+1): L.append(f"$wks.col{c}.name$"); L.append(f"$wks.col{c}.lname$"); L.append(f"$wks.col{c}.comment$")
    L.append("?wks.nCols"); L.append("?wks.nRows")
out=run(L,'mdwt2'); print(stat(out))
open('mdwt2.cases','w').write(repr(cases))

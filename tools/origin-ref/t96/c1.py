from lib import *
L=[]; n=64
x=[0.0]*n; x[32]=1.0
sc=[1.0,2.0,4.0,8.0]
book(L,'C1',[x,sc])
L.append("cwt ix1:=col(1) ix2:=col(2) type:=0 param:=5 upf:=0 plot:=0;")
L.append("$page.name$"); L.append("$wks.name$"); L.append("?page.nLayers"); L.append("?wks.nCols"); L.append("?wks.nRows")
L.append("#[C1]Sheet1"); 
for c in range(1,8): L.append(f"$wks.col{c}.name$"); L.append(f"$wks.col{c}.lname$"); L.append(f"$wks.col{c}.comment$")
L.append("#[C1]CWTResults1")
L.append("?wks.nCols"); L.append("?wks.nRows")
for c in range(1,8): L.append(f"$wks.col{c}.name$"); L.append(f"$wks.col{c}.lname$"); L.append(f"$wks.col{c}.comment$")
out=run(L,'cwt1'); print(stat(out)); print('\n'.join(l[:200] for l in out.splitlines() if l.startswith(('wks','page','[','?','#'))and 'col(' not in l)[:200])

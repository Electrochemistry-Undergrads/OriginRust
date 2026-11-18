from lib import *
L=[]; n=64
x=[0.0]*n; x[32]=1.0; sc=[2.0,4.0,8.0]
book(L,'PF',[x,sc])
L.append("cwt ix1:=col(1) ix2:=col(2) type:=0 param:=5 upf:=1 pf:=<new> mcoef:=<new> plot:=0;")
for q in ["$page.name$","$page.longname$","?page.type","?page.nLayers","$wks.name$","?wks.nCols","?wks.nRows","?mat.nCols","?mat.nRows","$mat.name$","$mat.xlabel$"]: L.append(q)
L.append("$wks.col1.name$"); 
L.append("win -a PF;")
L.append("$page.name$"); L.append("$wks.name$"); L.append("?wks.nCols")
L.append("#[PF]Sheet1")
for c in range(1,6): L.append(f"$wks.col{c}.name$"); L.append(f"$wks.col{c}.lname$");L.append(f"$wks.col{c}.comment$")
L.append("#[PF]CWTResults1")
# image plot
book(L,'IM',[x,sc]); L.append("cwt ix1:=col(1) ix2:=col(2) type:=2 plot:=1;")
for q in ["$page.name$","?page.type","?page.nLayers","$wks.name$"]: L.append(q)
L.append("doc -e P { $page.name$=%H; type -b \"%H|%(page.type)\"; }")
out=run(L,'cwt3'); print(stat(out)); print('\n'.join(l[:160] for l in out.splitlines() if not l.startswith('[True] col') and not re.match(r'^[-0-9.\t e]+$',l)))

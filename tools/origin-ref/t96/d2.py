from lib import *
L=[]
x16=sig(16); x21=sig(21)
for ty in range(21):
    for ext in (0,1):
        n=f"D{ty}_{ext}"; book(L,n,[x16])
        L.append(f"dwt ix:=col(1) type:={ty} ext:={ext} ca:=<new> cd:=<new>;")
        L.append(f"#[{n}]Sheet1")
        L.append(f"$wks.col2.name$"); L.append(f"$wks.col3.name$")
for ty in (0,3,13):
    n=f"E{ty}"; book(L,n,[x21]); L.append(f"dwt ix:=col(1) type:={ty} ext:=1 ca:=<new> cd:=<new>;"); L.append(f"#[{n}]Sheet1")
    n=f"F{ty}"; book(L,n,[x21]); L.append(f"dwt ix:=col(1) type:={ty} ext:=0 ca:=<new> cd:=<new>;"); L.append(f"#[{n}]Sheet1")
out=run(L,'dwt2'); print(stat(out)[:10]); print(out[-1500:])

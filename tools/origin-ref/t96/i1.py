from lib import *
from wt import *
L=[]; cases=[]
def case(nm,ca,cd,ty,ext):
    book(L,nm,[ca,cd]); L.append(f"idwt ca:=col(1) cd:=col(2) type:={ty} ext:={ext} ox:=<new>;"); L.append(f"#[{nm}]Sheet1")
for k in range(8):
    ca=[0.0]*8; ca[k]=1.0; case(f'IA{k}',ca,[0.0]*8,1,0)
    cd=[0.0]*8; cd[k]=1.0; case(f'ID{k}',[0.0]*8,cd,1,0)
for k in (0,3,8):
    ca=[0.0]*9; ca[k]=1.0; case(f'ZA{k}',ca,[0.0]*9,1,1)
    cd=[0.0]*9; cd[k]=1.0; case(f'ZD{k}',[0.0]*9,cd,1,1)
case('LEN1',[1,2,3,4,5],[1,2,3],1,0)
case('LEN2',[1,2,3,4,5],[1,2,3,4,5,6],1,1)
case('LEN3',[1,2,3,4,5],[1,2,3,4,5],1,0)
case('LEN4',[1,2,3,4,5],[1,2,3,4,5],1,1)
case('LEN5',[1,2,3,4,5],[1,2,3,4,5],0,1)
case('LEN6',[1,2,3,4,5],[1,2,3,4,5],3,0)
out=run(L,'idwt1'); print(stat(out))

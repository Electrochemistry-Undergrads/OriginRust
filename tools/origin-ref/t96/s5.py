from lib import *
from wt import *
L=[]; cases=[]
n=16
def mk(ca,d): return list(np.round(idwt_fix(np.array(ca,float),np.array(d,float),'db2',-2),9))
def add(nm,ca,d,thrd=50):
    y=mk(ca,d); cases.append((nm,ca,d,thrd))
    book(L,nm,[list(range(n)),y]); L.append(f"wtdenoise iy:=(1,2) type:=db2 ext:=periodic thrt:=custom level:=1 thrd:={thrd};"); L.append(f"#[{nm}]Sheet1")
z=[0]*8
for k in range(8):
    d=list(z); d[k]=10; add(f'K{k}',z,d)
d=list(z); d[0]=3; d[3]=10; add('KA',z,d)
d=list(z); d[7]=3; d[3]=10; add('KB',z,d)
d=list(z); d[3]=3; d[1]=10; add('KC',z,d)
d=list(z); d[1]=3; d[2]=10; add('KD',z,d)
a=list(z); a[0]=5; d=list(z); d[3]=10; d[0]=2; add('KE',a,d)
open('s5.cases','w').write(repr(cases))
out=run(L,'s51'); print(stat(out))
S=parse(out)
for (nm,ca,d,thrd) in cases:
    y=np.array(mk(ca,d)); o=np.array(S[f"[{nm}]Sheet1"][3])
    dt=np.where(abs(np.array(d,float))>=0.5*10,np.array(d,float),0)
    m=idwt_fix(np.array(ca,float),dt,'db2',-2)
    print(nm,'err idx',np.nonzero(abs(o-m)>1e-7)[0],np.round((o-m)[abs(o-m)>1e-7],4))

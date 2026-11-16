from lib import *; from wt import *
L=[]; ns=[64,128,256,512,1024,2048]
def yv(n): 
    i=np.arange(1,n+1); return 5*np.sin(i*i*0.3719)
for n in ns:
    L.append('newbook name:="R%d" sheet:=1;'%n)
    L.append(f"loop(ii,1,{n}) {{ col(1)[ii]=ii-1; col(2)[ii]=5*sin(ii*ii*0.3719); }}")
    L.append(f"range rr=col(2); ")
    L.append("wtdenoise iy:=(1,2) type:=db2 ext:=periodic thrt:=sqtwolog level:=1;"); L.append(f"#[R{n}]Sheet1")
out=run(L,'sq4'); print(stat(out)); S=parse(out)
for n in ns:
    k=f"[R{n}]Sheet1"
    if k not in S: print(n,'missing'); continue
    cols=S[k]; y=np.array(cols[1]); 
    if len(y)!=n: print(n,'len',len(y)); continue
    o=np.array(cols[3]); yc=yv(n); print(n,'input check',abs(y-yc).max())
    a,d=dwt_p(yc,'db2'); a2,d2=dwt_p(o,'db2'); nz=abs(d2)>1e-6; msk=np.arange(len(d))>0
    print(n,'T in [%.4f,%.4f)'%(abs(d[(~nz)&msk]).max(),abs(d[nz&msk]).min()),'ncoef',len(d))

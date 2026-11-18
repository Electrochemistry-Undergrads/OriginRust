from lib import *
from wt import *
import numpy as np
def design(n,name='db2',top=6.0,seed=1):
    rng=np.random.default_rng(seed)
    d=np.linspace(0.01,top,n//2)*rng.choice([-1,1],n//2); rng.shuffle(d)
    a=np.zeros(n//2)
    L=len(filt(name)[0]); y=idwt_fix(a,d,name,-(L-2))
    return np.round(y,9),d
if __name__=='__main__':
    L=[]; ns=[16,32,64,128,256,512,1024]; 
    for n in ns:
        y,d=design(n)
        book(L,f'T{n}',[list(range(n)),list(y)]); L.append("wtdenoise iy:=(1,2) type:=db2 ext:=periodic thrt:=sqtwolog level:=1;"); L.append(f"#[T{n}]Sheet1")
    out=run(L,'sq2'); print(stat(out))
    S=parse(out)
    for n in ns:
        y,d=design(n); o=np.array(S[f"[T{n}]Sheet1"][3]); a2,d2=dwt_p(o,'db2')
        nz=abs(d2)>1e-6
        print(n,'T in [%.4f,%.4f)'%(abs(d[~nz]).max() if (~nz).any() else 0,abs(d[nz]).min() if nz.any() else 99),'kept',nz.sum(),'/',len(d),'hard-eq',abs(d2[nz]-d[nz]).max() if nz.any() else '', 'a diff',abs(a2).max(), 'sqrt2ln',np.sqrt(2*np.log(n)))

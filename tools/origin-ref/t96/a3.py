from lib import *
import pywt, numpy as np
S=parse(open('dwt2.out').read()); x=np.array(sig(16)); n=16
names=['haar']+[f'db{i}' for i in range(2,11)]+['bior1.1','bior1.3','bior1.5','bior2.2','bior2.4','bior2.6','bior2.8','bior3.1','bior3.3','bior3.5','bior3.7']
def fit(f,target):
    best=None
    for off in range(-20,21):
      for conv in (1,-1):
        for sg in (1,-1):
            y=np.array([sum(f[j]*x[(2*k+off-conv*j)%n] for j in range(len(f))) for k in range(n//2)])*sg
            e=abs(y-target).max()
            if best is None or e<best[0]: best=(e,off,conv,sg)
    return best
for ty in range(21):
    ca=np.array(S[f"[D{ty}_0]Sheet1"][2]); cd=np.array(S[f"[D{ty}_0]Sheet1"][3])
    w=pywt.Wavelet(names[ty])
    ra=fit(np.array(w.dec_lo),ca); rd=fit(np.array(w.dec_hi),cd)
    print(ty,names[ty],len(w.dec_lo),'ca',ra,'cd',rd)

import numpy as np, pywt
def filt(name):
    w=pywt.Wavelet(name); return np.array(w.dec_lo),np.array(w.dec_hi),np.array(w.rec_lo),np.array(w.rec_hi)
def dwt_p(x,name):
    lo,hi,_,_=filt(name); n=len(x); L=len(lo)
    ca=np.array([sum(lo[j]*x[(2*k+1-j)%n] for j in range(L)) for k in range(n//2)])
    cd=np.array([sum(hi[j]*x[(2*k+1-j)%n] for j in range(L)) for k in range(n//2)])
    return ca,cd
def idwt_p(ca,cd,name,sh=None):
    _,_,rl,rh=filt(name); n=2*len(ca); L=len(rl)
    # candidate: x[m]=sum_k ca[k]*rl[(m-2k-SH) ] circular
    best=None
    out=None
    for SH in range(-L-2,L+3):
        y=np.zeros(n)
        for k in range(len(ca)):
            for j in range(L):
                y[(2*k+j+SH)%n]+=ca[k]*rl[j]+cd[k]*rh[j]
        yield SH,y
def idwt_fix(ca,cd,name,SH):
    _,_,rl,rh=filt(name); n=2*len(ca); L=len(rl); y=np.zeros(n)
    for k in range(len(ca)):
        for j in range(L):
            y[(2*k+j+SH)%n]+=ca[k]*rl[j]+cd[k]*rh[j]
    return y
def find_sh(name,n=16):
    x=np.sin(np.arange(n)*0.7)+np.arange(n)*0.1
    ca,cd=dwt_p(x,name)
    for SH,y in idwt_p(ca,cd,name):
        if abs(y-x).max()<1e-9: return SH
    return None
if __name__=='__main__':
    for nm in ['haar','db2','db4','bior1.3','bior2.2','bior3.7','db10']: print(nm,find_sh(nm))

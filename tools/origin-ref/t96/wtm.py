import numpy as np
from wt import *
def decomp(x,name,lv,ext=0):
    ds=[]; a=np.asarray(x,float)
    for l in range(lv):
        a,d=dwt_p(a,name); ds.append(d)
    return a,ds
def recon(a,ds,name):
    L=len(filt(name)[0]); SH=-(L-2)
    for d in reversed(ds): a=idwt_fix(a,d,name,SH)
    return a
def tailzero(yy,name,K,depth):
    a,ds=decomp(yy,name,depth)
    cat=np.concatenate([d for d in reversed(ds)])
    if K>0: cat[len(cat)-K:]=0
    dd=[None]*depth; p=0
    for l in reversed(range(depth)):
        dd[l]=cat[p:p+len(ds[l])]; p+=len(ds[l])
    return recon(a,dd,name)

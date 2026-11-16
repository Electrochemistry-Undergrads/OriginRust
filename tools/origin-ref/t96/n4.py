from lib import *
from n1 import noisy
from wt import *
cases=eval(open('dn.cases').read()); S=parse(open('dn1.out').read())
y=np.array(noisy(128))
def wnm(t): return {'haar':'haar','b13':'bior1.3','b37':'bior3.7'}.get(t,t)
def decomp(x,name,lv):
    ds=[]; a=x
    for l in range(lv):
        a,d=dwt_p(a,name); ds.append(d)
    return a,ds
def recon(a,ds,name):
    L=len(filt(name)[0]); SH=-(L-2)
    for d in reversed(ds): a=idwt_fix(a,d,name,SH)
    return a
def thr(d,t,mode):
    if mode=='soft': return np.sign(d)*np.maximum(abs(d)-t,0)
    return np.where(abs(d)>t,d,0)
for i in [1,2,3,5,6,7,9,10,11,21,22,23]:
    c=cases[i]; o=np.array(S[f"[W{i}]Sheet1"][3]); nm=wnm(c['type']); lv=c['level']
    a,ds=decomp(y,nm,lv)
    res=[]
    for mode in ('soft','hard'):
      for base in ('maxall','maxlvl','rmslvl'):
        for apply in ('all','finest'):
            ds2=[]
            for l,d in enumerate(ds):
                if apply=='finest' and l>0: ds2.append(d); continue
                m={'maxall':max(abs(dd).max() for dd in ds),'maxlvl':abs(d).max(),'rmslvl':np.sqrt(np.mean(d**2))}[base]
                ds2.append(thr(d,c['thrd']/100*m,mode))
            r=recon(a,ds2,nm); res.append((abs(r-o).max(),mode,base,apply))
    res.sort(); print(i,c['level'],c['thrd'],nm,res[:2])

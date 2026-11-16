import sys, numpy as np
from scipy import stats
sys.argv=['x','/dev/null']
exec(open('gp.py').read().split("run(\"a\"")[0])
def corr_pair(a,b):
    m=~(np.isnan(a)|np.isnan(b)); return np.corrcoef(a[m],b[m])[0,1], m.sum()
def arr(c): return np.array([np.nan if v is None else v for v in c],float)
def pcor(cols, vi, ci, mode):
    X=[arr(c) for c in cols]
    if mode=='list':
        m=np.ones(len(X[0]),bool)
        for i in vi+ci: m&=~np.isnan(X[i])
        X=[x[m] for x in X]
    res={}
    for a in range(len(vi)):
        for b in range(a+1,len(vi)):
            i,j=vi[a],vi[b]
            idx=[i,j]+ci
            k=len(idx)
            R=np.eye(k); N={}
            for p in range(k):
                for q in range(p+1,k):
                    r,n=corr_pair(X[idx[p]],X[idx[q]]); R[p,q]=R[q,p]=r; N[(p,q)]=n
            P=np.linalg.inv(R); r=-P[0,1]/np.sqrt(P[0,0]*P[1,1])
            n=N[(0,1)] ; df=n-2-len(ci)
            t=r*np.sqrt(df/(1-r*r)); p2=2*stats.t.sf(abs(t),df)
            res[(i,j)]=(r,p2,p2/2,df)
    return res
random_state=None
import random
random.seed(11)
c1=mk(20); c2=mk(20,[(0,3),(1,7),(2,11),(3,15),(1,16)]); c3=mk(8)
for name,c,vi,ci,mode in [('a',c1,[0,1],[2],'pair'),('b',c1,[0,1],[2,3],'pair'),('c',c1,[0,1,2],[3],'pair'),('d',c2,[0,1,2],[3],'pair'),('e',c2,[0,1,2],[3],'list'),('g',c3,[0,1],[2,3],'pair')]:
    print(name,{k:tuple(round(x,6) if abs(x)>1e-3 else float('%.5g'%x) for x in v) for k,v in pcor(c,vi,ci,mode).items()})

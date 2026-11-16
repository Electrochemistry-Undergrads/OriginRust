import sys, itertools, numpy as np
from scipy import stats
from g3 import *
def design(rows, terms):
    A=np.array([r[0] for r in rows]);B=np.array([r[1] for r in rows]);C=np.array([r[2] for r in rows]);y=np.array([r[3] for r in rows])
    def eff(v):
        lv=sorted(set(v)); cols=[]
        for l in lv[:-1]: cols.append(np.where(v==l,1.0,np.where(v==lv[-1],-1.0,0.0)))
        return np.array(cols).T
    E={'A':eff(A),'B':eff(B),'C':eff(C)}
    def term(t):
        m=np.ones((len(y),1))
        for ch in t:
            m=np.array([np.kron(m[i],E[ch][i]) for i in range(len(y))])
        return m
    return y,{t:term(t) for t in terms}
ALL=['A','B','C','AB','AC','BC','ABC']
def rss(y,X):
    X=np.column_stack([np.ones(len(y))]+X) if X else np.ones((len(y),1))
    b,res,rk,sv=np.linalg.lstsq(X,y,rcond=None); r=y-X@b; return r@r, np.linalg.matrix_rank(X)
def table(rows, terms=ALL):
    y,T=design(rows,terms)
    full,rk=rss(y,[T[t] for t in terms]); dfe=len(y)-rk
    out={}
    for t in terms:
        r2,rk2=rss(y,[T[u] for u in terms if u!=t]); out[t]=(rk-rk2, r2-full)
    # type I sequential
    seq={}; prev=rss(y,[])[0]; prk=1
    for i,t in enumerate(terms):
        r,rk_=rss(y,[T[u] for u in terms[:i+1]]); seq[t]=(rk_-prk,prev-r); prev=r; prk=rk_
    # type II
    t2={}
    def contains(a,b): return set(b)<=set(a)
    for t in terms:
        others=[u for u in terms if not contains(u,t)]  # terms not containing t
        r1,rk1=rss(y,[T[u] for u in others]); r2,rk2=rss(y,[T[u] for u in others]+[T[t]])
        t2[t]=(rk2-rk1,r1-r2)
    tot=((y-y.mean())**2).sum()
    return out,seq,t2,full,dfe,tot,len(y)
if __name__=='__main__':
    for name,cells in (('bal',BAL),('unb',UNB)):
        rows=three(cells)
        o,s,t2,full,dfe,tot,n=table(rows)
        print(name,'n',n,'SSE',round(full,5),'dfe',dfe,'SST',round(tot,5))
        for t in ALL: print(' ',t,'III',o[t][0],round(o[t][1],5),'  I',round(s[t][1],5),'  II',round(t2[t][1],5),' F3',round(o[t][1]/o[t][0]/(full/dfe),5),'p',stats.f.sf(o[t][1]/o[t][0]/(full/dfe),o[t][0],dfe))

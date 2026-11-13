from gen import *
import json, sys
from g2 import seq
from g3 import signpair
cases=[]
for n,npos in ((26,18),(27,19),(28,20),(26,13),(28,14)):
    x,y=signpair(n,npos); cases.append(('sign',f'sn{n}_{npos}',[x,y],dict(tail='two')))
x,y=signpair(32,20); 
cases.append(('sign','sn32ties4',[x[:26]+[5.0]*4,y[:26]+[5.0]*4],dict(tail='two')))
x,y=signpair(10,5)
for t in ('two','upper','lower'): cases.append(('sign','seq10_'+t,[x,y],dict(tail=t)))
x,y=signpair(30,15)
for t in ('two','upper','lower'): cases.append(('sign','seq30_'+t,[x,y],dict(tail=t)))
for n1,n2 in ((70,80),(80,100),(100,100),(90,110),(60,200),(50,250),(40,300),(65,75),(75,60)):
    for t in ('two',):
        cases.append(('ks',f'kk{n1}_{n2}',[seq(n1,1.1,20,2),seq(n2,0.8,20,2,1.5)],dict(tail=t)))
def script():
    out=""
    for kind,nm,cs,kw in cases:
        out+=cols(cs); nc=len(cs); rng=",".join(str(i+1) for i in range(nc)); al=0.05; sh='R'+nm
        if kind=='sign': out+=f"sign2 irng:=({rng}) null:=0 tail:={kw['tail']} alpha:={al} rt:=<new name:={sh}>;\n"+dump("%H",sh,16,5,nm)
        elif kind=='ks': out+=f"kstest2 type:=1 irng:=({rng}) tail:={kw['tail']} alpha:={al} rt:=<new name:={sh}>;\n"+dump("%H",sh,18,5,nm)
    return out
json.dump([dict(kind=k,name=n,cols=cs,**kw) for k,n,cs,kw in cases],open('cases4.json','w'))
open(sys.argv[1],'w').write(script())

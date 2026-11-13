from gen import *
import json, sys
from g2 import seq
from g3 import signpair
cases=[]
for n1,n2 in ((400,300),(250,50),(200,150),(120,100)):
    cases.append(('ks',f'kl{n1}_{n2}',[seq(n1,1.1,20,2),seq(n2,0.8,20,2,1.5)],dict(tail='two')))
cases.append(('ks','kl_up_300_400',[seq(300,1.1,20,1),seq(400,0.8,20,1,0.6)],dict(tail='upper')))
cases.append(('ks','kl_lo_300_400',[seq(300,1.1,20,1),seq(400,0.8,20,1,0.6)],dict(tail='lower')))
cases.append(('sign','sz_two',[[1.0,1.0,1.0],[1.0,1.0,1.0]],dict(tail='two')))
cases.append(('sign','sz_up',[[1.0,1.0,1.0],[1.0,1.0,1.0]],dict(tail='upper')))
cases.append(('sign','sz_low',[[1.0,1.0,1.0],[1.0,1.0,1.0]],dict(tail='lower')))
x,y=signpair(24,12)
for t in ('upper','lower'): cases.append(('sign','seq24_'+t,[x,y],dict(tail=t)))
def script():
    out=""
    for kind,nm,cs,kw in cases:
        out+=cols(cs); nc=len(cs); rng=",".join(str(i+1) for i in range(nc)); al=0.05; sh='R'+nm
        if kind=='sign': out+=f"sign2 irng:=({rng}) null:=0 tail:={kw['tail']} alpha:={al} rt:=<new name:={sh}>;\n"+dump("%H",sh,16,5,nm)
        elif kind=='ks': out+=f"kstest2 type:=1 irng:=({rng}) tail:={kw['tail']} alpha:={al} rt:=<new name:={sh}>;\n"+dump("%H",sh,18,5,nm)
    return out
json.dump([dict(kind=k,name=n,cols=cs,**kw) for k,n,cs,kw in cases],open('cases5.json','w'))
open(sys.argv[1],'w').write(script())

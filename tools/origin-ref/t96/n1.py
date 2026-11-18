from lib import *
import random
def noisy(n,seed=7):
    s=seed; out=[]
    for i in range(n):
        s=(1103515245*s+12345)%2147483648; u=s/2147483648-0.5
        out.append(round(math.sin(2*math.pi*i/32)+0.5*math.sin(2*math.pi*i/9)+0.8*u,6))
    return out
if __name__=='__main__':
    L=[]; x=list(range(128)); y=noisy(128)
    book(L,'W1',[x,y])
    L.append("wtdenoise iy:=(1,2) type:=db2 ext:=periodic thrt:=custom level:=1 thrd:=50;")
    L.append("#[W1]Sheet1")
    for c in range(1,7): L.append(f"$wks.col{c}.name$"); L.append(f"$wks.col{c}.lname$"); L.append(f"$wks.col{c}.comment$")
    L.append("?wks.nCols")
    out=run(L,'dn0'); print(stat(out)); 
    print('\n'.join(l for l in out.splitlines() if l.startswith(('[','wks'))and 'col(' not in l))

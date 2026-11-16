from lib import *; from n1 import noisy; from wt import *
L=[]; n=128; y=noisy(n); cuts=[0,5,10,20,30,50,70,90,100]
for c in cuts:
    book(L,f'SM{c}',[list(range(n)),y]); L.append(f"wtsmooth iy:=(1,2) type:=db2 ext:=periodic cutoff:={c};"); L.append(f"#[SM{c}]Sheet1"); L.append("$wks.col4.comment$"); L.append("$wks.col4.lname$")
out=run(L,'sm1'); print(stat(out)); S=parse(out)
print([l for l in out.splitlines() if 'comment' in l][:3])
yy=np.array(y)
from n4 import decomp
for c in cuts:
    o=np.array(S[f"[SM{c}]Sheet1"][3]); 
    a,ds=decomp(yy,'db2',6); a2,ds2=decomp(o,'db2',6)
    print(c,'rms diff %.4f'%np.sqrt(np.mean((o-yy)**2)),'approx diff',round(abs(a-a2).max(),6),'kept per level',[(int((abs(d2)>1e-9).sum()),len(d2)) for d2 in ds2])

ns=[5,6,8,10,12,15,20,25,30,40,50,60,80,100]
L=['newbook;']
c=0
meta=[]
for n in ns:
    for v in range(5):
        c+=1
        if c>1: L.append('wks.addcol();')
        a=1+0.7*v; b=0.9+0.37*v+0.013*n; cc=1.3+0.5*(v%3); d=0.21+0.17*v
        L.append(f'col({c}) = data(1,{n},1); col({c}) = {a}*sin({b}*col({c})) + {cc}*cos({d}*col({c})^1.3) + {0.05*v}*col({c});')
        meta.append((c,n))
L.append('double s, d, p;')
for c,n in meta:
    L.append(f'kstest irng:=col({c}) stat:=s df:=d prob:=p;')
    L.append(f'?s'); L.append('?d'); L.append('?p')
    L.append(f'lillietest irng:=col({c}) stat:=s df:=d prob:=p;')
    L.append(f'?s'); L.append('?d'); L.append('?p')
L.append('#[Book1]Sheet1')
open('ks-probe.lines.txt','w').write('\n'.join(L)+'\n')
print(len(L))

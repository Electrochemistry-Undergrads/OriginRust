import math, json
def y(i,a,b): return round(10 + 1.5*a + 0.8*b + 0.6*((a*b)%3) + 2*math.sin(7*i), 4)
def build(cells):  # cells: dict (a,b)->n
    rows=[]; i=0
    for (a,b),n in cells.items():
        for _ in range(n):
            i+=1; rows.append((a,b,y(i,a,b)))
    return rows
cases=[]
bal={(a,b):3 for a in (1,2) for b in (1,2,3)}
unb={(1,1):3,(1,2):2,(1,3):4,(2,1):1,(2,2):3,(2,3):2}
# interleave rows so that order of first appearance is not trivially sorted: shuffle deterministic
def shuffle(rows):
    r=rows[:]; out=[]; k=0
    idx=list(range(len(r)))
    order=sorted(idx,key=lambda j:(j*7)%len(r)*1000+j)
    return [r[j] for j in order]
cases.append(dict(name="bal",rows=shuffle(build(bal)),nA=2,nB=3,runs=[(1,2),(1,1),(1,3),(1,4),(1,5),(0,2)]))
cases.append(dict(name="unb",rows=shuffle(build(unb)),nA=2,nB=3,runs=[(1,2),(1,1),(1,3),(1,4),(1,5),(0,2),(0,5)]))
unb2={(1,1):2,(1,2):5,(2,1):4,(2,2):1,(3,1):3,(3,2):3}
cases.append(dict(name="unb32",rows=shuffle(build(unb2)),nA=3,nB=2,runs=[(1,2),(0,2),(1,6),(1,7)]))
emptycell={(1,1):3,(1,2):2,(1,3):2,(2,1):3,(2,2):0,(2,3):3}
cases.append(dict(name="empty",rows=shuffle(build(emptycell)),nA=2,nB=3,runs=[(1,2),(0,2)]))
ones={(a,b):1 for a in (1,2) for b in (1,2,3)}
cases.append(dict(name="ones",rows=shuffle(build(ones)),nA=2,nB=3,runs=[(1,2),(0,2)]))
const={(a,b):2 for a in (1,2) for b in (1,2)}
rows=[(a,b,5.0) for (a,b),n in const.items() for _ in range(n)]
cases.append(dict(name="const",rows=rows,nA=2,nB=2,runs=[(1,2),(0,2)]))
lines=["newbook;","int rc = run.LoadOC(%WORK%stats2-ref.c, 16);","?rc"]
bk=0
for c in cases:
    bk+=1
    if bk>1: lines.append("newbook;")
    A=", ".join(str(r[0]) for r in c['rows']); B=", ".join(str(r[1]) for r in c['rows']); C=", ".join(repr(r[2]) for r in c['rows'])
    lines.append(f"col(A) = {{{A}}};")
    lines.append(f"wks.addcol(); col(B) = {{{B}}};")
    lines.append(f"wks.addcol(); col(C) = {{{C}}};")
    c['book']=bk
    for k,(inter,m) in enumerate(c['runs']):
        lines.append(f"double r{bk}_{k} = sheaf_anova2({inter}, {c['nA']}, {c['nB']}, {m}, 0.05);")
        lines.append(f"?r{bk}_{k}")
        lines.append(f"#[Book{bk}]Sheet1")
open("anova2.lines.txt","w").write("\n".join(lines)+"\n")
json.dump(cases,open("anova2.cases.json","w"))
print(len(lines))

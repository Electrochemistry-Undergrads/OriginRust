import json, math
def seq(n, seed, scale=10.0, nd=1, shift=0.0):
    # deterministic closed-form numbers (sines), rounded to force ties when nd is small
    return [round(shift + scale*(0.5+0.5*math.sin(seed*i + 0.3*seed*i*i/(i+3.0))), nd) for i in range(1, n+1)]
cases=[]
def add(kind,name,cols,**kw): cases.append(dict(kind=kind,name=name,cols=cols,**kw))
# ---------------- Mann-Whitney
a=[1.1,2.3,2.3,4.0,5.5,6.1,7.2,7.2,9.9]; b=[2.2,3.1,4.0,4.0,8.8,10.5,11.0]
for tail in ('two','upper','lower'):
    for ex in (0,1): add('mw',f'mw1_{tail}_{ex}',[a,b],tail=tail,exact=ex)
a2=[1.2,3.4,2.2,9.1,5.7,8.3]; b2=[4.1,6.6,7.3,10.2,12.0,11.4,5.0,13.3]
for tail in ('two','upper','lower'):
    for ex in (0,1): add('mw',f'mw2_{tail}_{ex}',[a2,b2],tail=tail,exact=ex)
a3=seq(14,1.3,10,0); b3=seq(16,0.9,10,0,1.5)
for ex in (0,1): add('mw',f'mw3_two_{ex}',[a3,b3],tail='two',exact=ex)
a4=seq(25,2.1,20,1); b4=seq(30,1.7,20,1,2.0)
add('mw','mw4_two_0',[a4,b4],tail='two',exact=0)
add('mw','mw4_two_1',[a4,b4],tail='two',exact=1)
add('mw','mw5_missing',[a[:4]+[None]+a[4:],b[:2]+[None]+b[2:]],tail='two',exact=0)
add('mw','mw6_const',[[5.0]*4,[5.0]*5],tail='two',exact=0)
add('mw','mw6_const_exact',[[5.0]*4,[5.0]*5],tail='two',exact=1)
add('mw','mw7_one',[[3.5],[1.0,2.0,4.0,5.0,6.0]],tail='two',exact=0)
add('mw','mw7_one_exact',[[3.5],[1.0,2.0,4.0,5.0,6.0]],tail='two',exact=1)
add('mw','mw8_alpha',[a,b],tail='two',exact=0,alpha=0.1)
add('mw','mw9_sep',[[1.0,2.0,3.0],[4.0,5.0,6.0,7.0]],tail='two',exact=1)
add('mw','mw9_sep0',[[1.0,2.0,3.0],[4.0,5.0,6.0,7.0]],tail='two',exact=0)
# ---------------- signed rank, one sample (median m)
s1=[1.1,2.3,2.3,4.0,5.5,6.1,7.2,7.2,9.9]
for tail in ('two','upper','lower'): add('sr1',f'sr1a_{tail}',[s1],tail=tail,median=3.0)
for nm,n in (('sr1b_n20',20),('sr1c_n30',30),('sr1d_n50',50),('sr1e_n100',100)):
    add('sr1',nm,[[round(3*math.sin(1.7*i)+0.4*i/3,2) for i in range(1,n+1)]],tail='two',median=0.0)
add('sr1','sr1f_zeros',[[1.0,2.0,3.0,3.0,4.0,5.0,3.0,6.5,0.5,2.5]],tail='two',median=3.0)
add('sr1','sr1g_ties',[[1.0,2.0,3.0,3.0,5.0,5.0,4.0,6.5,0.5,2.0,7.5,8.5]],tail='two',median=3.0)
add('sr1','sr1h_const',[[3.0]*5],tail='two',median=3.0)
add('sr1','sr1i_n1',[[4.0]],tail='two',median=3.0)
add('sr1','sr1j_n2',[[4.0,5.0]],tail='two',median=3.0)
add('sr1','sr1k_n5',[[1.0,2.0,3.5,4.5,6.0]],tail='two',median=3.0)
add('sr1','sr1l_missing',[[1.1,None,2.3,4.0,5.5,6.1,None,7.2,9.9]],tail='two',median=3.0)
# paired
pa=[1.1,2.3,2.3,4.0,5.5,6.1,7.2,7.2,9.9]; pb=[2.2,3.1,4.0,4.0,8.8,10.5,11.0,12.5,12.9]
for tail in ('two','upper','lower'): add('sr2',f'sr2a_{tail}',[pa,pb],tail=tail)
pc=[round(5+3*math.sin(1.3*i),1) for i in range(1,16)]; pd=[round(5.4+3*math.sin(1.3*i+0.5)+0.2*i/5,1) for i in range(1,16)]
add('sr2','sr2b_ties',[pc,pd],tail='two')
add('sr2','sr2c_n40',[[round(5+3*math.sin(1.3*i),2) for i in range(1,41)],[round(5.4+3*math.sin(1.3*i+0.5)+0.2*i/5,2) for i in range(1,41)]],tail='two')
add('sr2','sr2d_identical',[pa,pa],tail='two')
add('sr2','sr2e_missing',[[1.1,None,2.3,4.0,5.5,6.1,7.2,7.2,9.9],[2.2,3.1,None,4.0,8.8,10.5,11.0,12.5,12.9]],tail='two')
# ---------------- Kruskal-Wallis
g1=[1.1,2.3,2.3,4.0,5.5,6.1,7.2,7.2,9.9]; g2=[2.2,3.1,4.0,4.0,8.8,10.5,11.0]; g3=[3.3,1.1,5.0,2.2,8.0,7.5,1.0,9.1,4.4,6.0,12.5]
add('kw','kw1',[g1,g2,g3])
g4=[0.5,1.5,2.5,2.5]
add('kw','kw2_four',[g1,g2,g3,g4])
add('kw','kw3_two',[g1,g2])
add('kw','kw4_const',[[5.0]*3,[5.0]*4,[5.0]*2])
add('kw','kw5_one',[g1,[4.0],g3])
add('kw','kw6_missing',[g1[:3]+[None]+g1[3:],g2,g3[:5]+[None]+g3[5:]])
add('kw','kw7_noties',[[1.2,3.4,2.2,9.1,5.7,8.3],[4.1,6.6,7.3,10.2,12.0,11.4,5.0,13.3],[0.3,15.2,14.1]])
add('kw','kw8_big',[seq(20,1.1,30,1),seq(25,0.7,30,1,3),seq(18,1.9,30,1,6)])
# ---------------- Correlation
c1=[1.1,2.3,2.3,4.0,5.5,6.1,7.2,7.2,9.9,10.4,11.0,12.2]
c2=[2.2,3.1,4.0,4.0,8.8,10.5,11.0,12.5,12.9,13.0,15.1,14.2]
c3=[3.3,1.1,5.0,2.2,8.0,7.5,1.0,9.1,4.4,6.0,12.5,2.2]
c4=[5.0,4.0,4.0,3.0,3.0,3.0,2.0,2.0,1.0,1.0,0.0,0.5]
add('cc','cc1',[c1,c2,c3,c4])
add('cc','cc2_missing_pairwise',[c1[:3]+[None]+c1[4:],c2,c3[:7]+[None]+c3[8:],c4],missing='pairwise')
add('cc','cc2_missing_listwise',[c1[:3]+[None]+c1[4:],c2,c3[:7]+[None]+c3[8:],c4],missing='listwise')
add('cc','cc3_perfect',[[1.0,2.0,3.0,4.0,5.0],[2.0,4.0,6.0,8.0,10.0],[5.0,4.0,3.0,2.0,1.0]])
add('cc','cc4_const',[c1,[3.0]*12])
add('cc','cc5_n3',[[1.0,2.0,4.0],[2.0,1.0,5.0]])
add('cc','cc6_n2',[[1.0,2.0],[2.0,5.0]])
add('cc','cc7_n20',[seq(20,1.1,10,2),seq(20,0.9,10,2,1),seq(20,2.3,10,2)])
add('cc','cc8_negative',[c1,[20-x for x in c1]])

# thresholds of exact tests
for n1,n2 in ((30,35),(40,45),(50,55),(60,65)):
    add('mw',f'mwT_{n1}_{n2}',[seq(n1,1.37,40,0),seq(n2,0.83,40,0,3)],tail='two',exact=1)
for n in (60,70,80,90):
    add('sr1',f'srT_{n}',[[round(2.5*math.sin(1.7*i)+0.12*i/3,2) for i in range(1,n+1)]],tail='two',median=0.0)
add('sr1','srT_ties40',[[round(2.5*math.sin(1.7*i)+0.12*i/3,0) for i in range(1,41)]],tail='two',median=0.0)
add('sr2','sr2f_n50',[[round(5+3*math.sin(1.3*i),2) for i in range(1,51)],[round(5.1+3*math.sin(1.3*i+0.5)+0.2*i/5,2) for i in range(1,51)]],tail='two')
add('sr2','sr2g_n80',[[round(5+3*math.sin(1.3*i),2) for i in range(1,81)],[round(5.1+3*math.sin(1.3*i+0.5)+0.2*i/5,2) for i in range(1,81)]],tail='two')
json.dump(cases,open('np.cases.json','w'))
lines=["newbook;"]
bk=0
for c in cases:
    bk+=1
    if bk>1: lines.append("newbook;")
    cols=c['cols']
    for j,col in enumerate(cols):
        present=[("0" if x is None else repr(float(x))) for x in col]
        lines.append(("wks.addcol(); " if j>0 else "")+f"col({j+1}) = {{{', '.join(present)}}};")
        for i,x in enumerate(col):
            if x is None: lines.append(f"col({j+1})[{i+1}] = 1/0;")
    n=len(cols)
    k=c['kind']; sh=f"S{bk}"
    if k=='mw':
        al=f" alpha:={c['alpha']}" if 'alpha' in c else ''
        lines.append(f"mwtest type:=1 irng:=(1,2) tail:={c['tail']} exact:={c['exact']}{al} rt:=<new name:={sh}>;")
    elif k=='sr1':
        lines.append(f"signrank1 irng:=col(1) median:={c['median']} tail:={c['tail']} rt:=<new name:={sh}>;")
    elif k=='sr2':
        lines.append(f"signrank2 irng:=(col(1),col(2)) tail:={c['tail']} rt:=<new name:={sh}>;")
    elif k=='kw':
        lines.append(f"kwanova type:=1 irng:=({','.join(str(j+1) for j in range(n))}) rt:=<new name:={sh}>;")
    elif k=='cc':
        rng=",".join(f"col({j+1})" for j in range(n))
        ms=f" missing:={c['missing']}" if 'missing' in c else ''
        lines.append(f"corrcoef irng:=({rng}) pearson:=1 spearman:=1 kendall:=1 flag:=1{ms} rt:=<new name:={sh}>;")
    lines.append(f"#[Book{bk}]{sh}")
open('np.lines.txt','w').write("\n".join(lines)+"\n")
print(bk,len(lines))

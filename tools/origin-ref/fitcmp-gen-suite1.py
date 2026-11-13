import json
N=20
data={
 'e':"1 + 3*exp(-col(A)/2) + 0.05*sin(37*col(D)) + 0.04*sin(101.3*col(D))",
 'g':"1 + 3*exp(-2*((col(A)-5)/1.5)^2) + 0.05*sin(37*col(D)) + 0.04*sin(101.3*col(D))",
 'l':"2 + 0.5*col(A) + 0.3*sin(3*col(A)) + 0.05*sin(37*col(D))",
}
cases=[
 dict(name='e_1v2_aic_only',d='e',f1='ExpDec1',f2='ExpDec2',opt='aic:=1 bic:=0 ftest:=0 param:=1 statics:=1'),
 dict(name='e_2v1_all',d='e',f1='ExpDec2',f2='ExpDec1',opt='aic:=1 bic:=1 ftest:=1 param:=1 statics:=1'),
 dict(name='g_gauss_lorentz',d='g',f1='Gauss',f2='Lorentz',opt='aic:=1 bic:=1 ftest:=1 param:=1 statics:=1'),
 dict(name='l_line_parab_w',d='l',f1='Line',f2='Parabola',w=True,opt='aic:=1 bic:=1 ftest:=1 param:=1 statics:=1'),
 dict(name='e_names_nopar',d='e',f1='ExpDec1',f2='ExpDec2',opt='aic:=1 bic:=1 ftest:=1 param:=0 statics:=0 name1:=Alpha name2:=Beta'),
 dict(name='l_line_parab',d='l',f1='Line',f2='Parabola',opt='aic:=0 bic:=1 ftest:=1 param:=1 statics:=1'),
 dict(name='l_line_cubic',d='l',f1='Line',f2='Cubic',opt='aic:=1 bic:=1 ftest:=1 param:=1 statics:=1'),
]
L=[]
for k,c in enumerate(cases,1):
    L+=["newbook;","col(A) = data(0.5, 10, 0.5);","col(B) = col(A)*0;","col(C) = col(A)*0;","col(D) = data(1, 20, 1);",
        "col(B) = %s;"%data[c['d']]]
    if c.get('w'): L.append("col(C) = 0.05 + 0.02*col(A);")
    iy="(1,2,3)" if c.get('w') else "(1,2)"
    wt=" weight:=ins" if c.get('w') else ""
    for fn in (c['f1'],c['f2']):
        L+=["nlbegin iy:=%s func:=%s nltree:=tt%s;"%(iy,fn,wt),"nlfit;","nlend output:=1;"]
    L+=["fitcmpmodel result1:=[Book%d]FitNL1 result2:=[Book%d]FitNL2 %s rt:=<new name:=cmpA>;"%(k,k,c['opt']),
        "#[Book%d]Sheet1"%k,"#[Book%d]FitNL1"%k,"#[Book%d]FitNL2"%k,"#[Book%d]cmpA"%k]
open('cm2.lines.txt','w').write("\n".join(L)+"\n")
json.dump(cases,open('cm2-meta.json','w'),indent=1)

"""Writes nlfit-surface-suite.lines.txt and nlfit-surface-meta.json: Origin surface-fit reference runs
(nlbeginz on XYZ worksheet columns, synthetic data made inside Origin from sine 'noise')."""
import json, sys

# name, params (true values), z-formula (LabTalk over col(A)=x col(B)=y), grid (nx, ny, x0, dx, y0, dy), noise amplitude, missing rows, weight
NOISE = "({a}*sin(37*col(D)) + {b}*sin(101.3*col(D)))"
cases = [
    dict(func="Plane", names="z0,a,b", f="1.5 + 0.8*col(A) - 0.6*col(B)", grid=(11, 11, 0, 0.5, 0, 0.5), amp=(0.03, 0.02)),
    dict(func="Parabola2D", names="z0,a,b,c,d", f="2 + 0.5*col(A) - 0.4*col(B) + 0.3*col(A)^2 + 0.2*col(B)^2", grid=(11, 11, -2.5, 0.5, -2.5, 0.5), amp=(0.04, 0.03)),
    dict(func="Poly2D", names="z0,a,b,c,d,f", f="1 + 0.5*col(A) - 0.4*col(B) + 0.3*col(A)^2 + 0.2*col(B)^2 + 0.35*col(A)*col(B)", grid=(11, 11, -2.5, 0.5, -2.5, 0.5), amp=(0.04, 0.03)),
    dict(func="Gauss2D", names="z0,A,xc,w1,yc,w2", f="0.4 + 3*exp(-0.5*((col(A)-2.6)/1.1)^2 - 0.5*((col(B)-2.2)/0.9)^2)", grid=(11, 11, 0, 0.5, 0, 0.5), amp=(0.03, 0.02)),
    dict(func="Gauss2D", names="z0,A,xc,w1,yc,w2", f="0.4 + 3*exp(-0.5*((col(A)-2.6)/1.1)^2 - 0.5*((col(B)-2.2)/0.9)^2)", grid=(11, 11, 0, 0.5, 0, 0.5), amp=(0.03, 0.02), missing=[5, 17, 40, 61, 90], tag="missing z"),
    dict(func="Lorentz2D", names="z0,A,xc,w1,yc,w2", f="0.3 + 2.5/((1+((col(A)-2.4)/1.2)^2)*(1+((col(B)-2.8)/0.8)^2))", grid=(11, 11, 0, 0.5, 0, 0.5), amp=(0.02, 0.015)),
    dict(func="Voigt2D", names="z0,A,xc,w1,yc,w2,mu", f="0.2 + 2.8*(0.4/((1+((col(A)-2.5)/1.3)^2)*(1+((col(B)-2.5)/1.0)^2)) + 0.6*exp(-0.5*((col(A)-2.5)/1.3)^2 - 0.5*((col(B)-2.5)/1.0)^2))", grid=(11, 11, 0, 0.5, 0, 0.5), amp=(0.02, 0.015)),
    dict(func="Gaussian2D", names="z0,A,xc,w1,yc,w2,theta", f="0.4 + 3*exp(-0.5*((col(A)*cos(0.5)+col(B)*sin(0.5)-2.6*cos(0.5)-2.2*sin(0.5))/1.3)^2 - 0.5*((-col(A)*sin(0.5)+col(B)*cos(0.5)+2.6*sin(0.5)-2.2*cos(0.5))/0.7)^2)", grid=(11, 11, 0, 0.5, 0, 0.5), amp=(0.03, 0.02)),
    dict(func="Exponential2D", names="z0,B,C,D", f="0.5 + 4*exp(-col(A)/2.5)*exp(-col(B)/3.5)", grid=(11, 11, 0.5, 0.5, 0.5, 0.5), amp=(0.02, 0.015)),
    dict(func="Rational2D", names="z0,A01,B01,B02,B03,A1,A2,A3,B1,B2", f="(1 + 0.3*col(A) + 0.2*col(B) + 0.05*col(B)^2 + 0.01*col(B)^3)/(1 + 0.1*col(A) + 0.02*col(A)^2 + 0.005*col(A)^3 + 0.15*col(B) + 0.03*col(B)^2)", grid=(11, 11, 0, 0.5, 0, 0.5), amp=(0.01, 0.008)),
    dict(func="Plane", names="z0,a,b", f="1.5 + 0.8*col(A) - 0.6*col(B)", grid=(9, 9, 0, 0.5, 0, 0.5), amp=(0.03, 0.02), missing=[3, 30], missing_x=[8], tag="missing x, z"),
]

L = []
meta = []
for k, c in enumerate(cases, 1):
    nx, ny, x0, dx, y0, dy = c["grid"]
    n = nx * ny
    names = c["names"].split(",")
    L.append("newbook;")
    L.append(f"col(A) = data(1, {n}, 1);")
    L += ["col(B) = col(A);", "col(C) = col(A);", "col(D) = col(A);"]
    L.append(f"col(A) = (col(D)-1) - {nx}*int((col(D)-1)/{nx});")
    L.append(f"col(A) = {x0} + col(A)*{dx};")
    L.append(f"col(B) = {y0} + int((col(D)-1)/{nx})*{dy};")
    L.append("col(C) = " + c["f"] + " + " + NOISE.format(a=c["amp"][0], b=c["amp"][1]) + ";")
    for r in c.get("missing", []):
        L.append(f"col(C)[{r}] = -1.23456789E-300;")
    for r in c.get("missing_x", []):
        L.append(f"col(A)[{r}] = -1.23456789E-300;")
    L.append(f"nlbeginz iz:=(1,2,3) func:={c['func']} nltree:=tt ;")
    for nm in names:
        L.append(f"double i_{nm} = tt.{nm};")
        L.append(f"?i_{nm}")
    L.append("nlfit;")
    L.append("nlend output:=1;")
    for nm in names:
        L.append(f"double v_{nm} = tt.{nm};")
        L.append(f"?v_{nm}")
    L.append("double nit = tt.niter;")
    L.append("?nit")
    L.append(f"#[Book{k}]Sheet1")
    L.append(f"#[Book{k}]FitSurface1")
    meta.append(dict(func=c["func"], params=names, tag=c.get("tag", ""), grid=list(c["grid"])))
open("nlfit-surface-suite.lines.txt", "w").write("\n".join(L) + "\n")
json.dump(meta, open("nlfit-surface-meta.json", "w"), indent=1)
print(len(cases), "cases", len(L), "lines")

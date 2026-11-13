"""Writes the Origin script (lines format) and the case list for the quick-fit functions
(Exponential: ExpGrow1; Sigmoidal: Logistic, Hill, DoseResp), run like nlfit-suite.lines.txt:
  python nlfit-quick-gen.py nlfit-quick.lines.txt nlfit-quick-meta.json
  run-labtalk-lines.ps1 -Script nlfit-quick.lines.txt > nlfit-quick.out
  python nlfit-make-fixture.py nlfit-quick.out nlfit-quick-meta.json out.json
"""
import json, sys

D = "col(D)"
noise = "0.02*sin(37*col(D)) + 0.015*sin(101.3*col(D))"
cases = [
    dict(name="expgrow1", func="ExpGrow1", params=["y0", "x0", "A1", "t1"], fixed={"x0": 0.0},
         x="data(0.5, 10, 0.5)", n=20, y="0.5 + 0.1*exp(col(A)/2.5) + " + noise),
    dict(name="logistic", func="Logistic", params=["A1", "A2", "x0", "p"],
         x="data(0.5, 10, 0.5)", n=20, y="3 + (0.2-3)/(1+(col(A)/4)^3) + " + noise),
    dict(name="hill", func="Hill", params=["Vmax", "k", "n"],
         x="data(0.5, 10, 0.5)", n=20, y="3*col(A)^2.5/(4^2.5+col(A)^2.5) + " + noise),
    dict(name="doseresp", func="DoseResp", params=["A1", "A2", "LOGx0", "p"],
         x="data(-2, 2, 0.1)", n=41, y="0.1 + (3-0.1)/(1+10^((0.3-col(A))*1.5)) + " + noise),
]
L = []
for c in cases:
    L += ["newbook;", "col(A) = %s;" % c["x"], "col(B) = col(A)*0;", "col(C) = col(A)*0;",
          "col(D) = data(1, %d, 1);" % c["n"], "col(B) = %s;" % c["y"],
          "nlbegin iy:=(1,2) func:=%s nltree:=tt ;" % c["func"]]
    for k, v in c.get("fixed", {}).items():
        L.append("tt.%s = %r; tt.f_%s = 1;" % (k, v, k))
    for p in c["params"]:
        L += ["double i_%s = tt.%s;" % (p, p), "?i_%s" % p]
    L += ["nlfit;", "nlend output:=1;"]
    for p in c["params"]:
        L += ["double v_%s = tt.%s;" % (p, p), "?v_%s" % p]
    L += ["double nit = tt.niter;", "?nit"]
    # book number is filled in by the order of the cases
L2 = []
k = 0
for l in L:
    L2.append(l)
    if l == "?nit":
        k += 1
        L2 += ["#[Book%d]Sheet1" % k, "#[Book%d]FitNL1" % k, "$page.name$"]
open(sys.argv[1], "w").write("\n".join(L2) + "\n")
meta = [{kk: c[kk] for kk in ("name", "func", "params", "fixed") if kk in c} for c in cases]
json.dump(meta, open(sys.argv[2], "w"), indent=1)

"""Multiple linear regression reference runs (`fitMR`) and the fixture maker.

  python fitmr-gen.py gen  fitmr.lines.txt
  run-labtalk-safe.ps1 -Script fitmr.lines.txt > fitmr.out
  python fitmr-gen.py make fitmr.out fitmr.lines.txt origin_fitmr.json

`fitMR dep:=... indep:=... mrtree:=mrt` fills the tree with v1.. (values: the intercept first, then one per
independent variable; a fixed intercept is stored as its value), e1.. (standard errors), t1.. and p1..
(t-values and Prob>|t|), SSE (residual sum of squares), SST (total, about the mean; about zero with a
fixed intercept), ssr (regression sum of squares), MSR, MSE, RMSE, DOF, COD, Rvalue, Npts, Fvalue and
PValue (Prob>F). With a fixed intercept `e1`, `t1` and `p1` are not set.
"""
import json
import re
import sys

N = 20
CASES = [
    dict(name="two_free", n=20, nx=2, fix=None),
    dict(name="three_free", n=25, nx=3, fix=None),
    dict(name="two_fix0", n=20, nx=2, fix=0.0),
    dict(name="two_fix1p5", n=20, nx=2, fix=1.5),
    dict(name="one_free", n=15, nx=1, fix=None),
    dict(name="three_fix_neg", n=30, nx=3, fix=-2.0),
]
COLS = "ABCD"
KEYS = "SSE SST MSR MSE RMSE DOF ssr COD Rvalue Npts Fvalue PValue".split()


def gen(out):
    L = []
    for k, c in enumerate(CASES, 1):
        n, nx = c["n"], c["nx"]
        L += ["newbook;", "col(A) = data(1, %d, 1);" % n]
        for j in range(1, nx):
            L.append("col(%s) = col(A)*0;" % COLS[j])
        y = COLS[nx]
        L.append("col(%s) = col(A)*0;" % y)
        if nx >= 2:
            L.append("col(B) = 0.5*col(A) + 3*sin(col(A)*1.7);")
        if nx >= 3:
            L.append("col(C) = 4*cos(col(A)*0.9) + 0.1*col(A);")
        terms = ["0.7*col(A)"] + ["%g*col(%s)" % (0.3 * (j + 1) * (-1) ** j, COLS[j]) for j in range(1, nx)]
        L.append("col(%s) = 2 + %s + 0.8*sin(col(A)*5.3) + 0.5*cos(col(A)*11.1);" % (y, " + ".join(terms)))
        indep = "(%s)" % ",".join(COLS[:nx]) if nx > 1 else "A"
        fx = " fixint:=1 intercept:=%r" % c["fix"] if c["fix"] is not None else ""
        L += ["tree mrt;", "fitMR dep:=[Book%d]Sheet1!%s indep:=[Book%d]Sheet1!%s mrtree:=mrt%s;" % (k, y, k, indep, fx)]
        for i in range(1, nx + 2):
            for p in "vetp":
                L += ["double r_%s%d = mrt.%s%d;" % (p, i, p, i), "?r_%s%d" % (p, i)]
        for key in KEYS:
            L += ["double r_%s = mrt.%s;" % (key, key), "?r_%s" % key]
        L.append("#[Book%d]Sheet1" % k)
    open(out, "w").write("\n".join(L) + "\n")


def make(src, lines_file, out):
    lines = open(src, encoding="utf8").read().split("\n")
    res = []
    k = 0
    cur = {}
    i = 0
    while i < len(lines):
        l = lines[i]
        m = re.match(r"r_(\w+) = (\S+)$", l)
        if m:
            v = float(m.group(2))
            cur[m.group(1)] = None if abs(v + 1.23456789e-300) < 1e-310 else v
        m = re.match(r"#SHEET (\S+) (\d+) (\d+)", l)
        if m:
            nrows = int(m.group(2))
            rows = [r.split("\t") for r in lines[i + 1:i + 1 + nrows]]
            c = dict(CASES[k])
            nx = c["nx"]
            c["x"] = [[float(r[j]) for r in rows] for j in range(nx)]
            c["y"] = [float(r[nx]) for r in rows]
            c["origin"] = cur
            res.append(c)
            cur = {}
            k += 1
            i += nrows
        i += 1
    json.dump(res, open(out, "w"), indent=1)
    print(len(res), "cases")


if sys.argv[1] == "gen":
    gen(sys.argv[2])
else:
    make(sys.argv[2], sys.argv[3], sys.argv[4])

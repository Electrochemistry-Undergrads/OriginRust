"""Writes the line script of the partial correlation (pcorrcoef) reference runs of T81.
usage: python gp.py OUT.lines.txt ; run with ../run-labtalk-safe.ps1. Own generator, own data."""
import sys, random
sys.path.insert(0, '../stats3')
from gen import cols

random.seed(11)


def mk(n, miss=()):
    z = [random.gauss(0, 1) for _ in range(n)]
    w = [random.gauss(0, 1) for _ in range(n)]
    x = [round(2 + z[i] + 0.6 * w[i] + random.gauss(0, 0.7), 3) for i in range(n)]
    y = [round(5 + 0.8 * z[i] - 0.5 * w[i] + 0.5 * x[i] + random.gauss(0, 0.8), 3) for i in range(n)]
    c = [x, y, [round(v, 3) for v in z], [round(v, 3) for v in w]]
    for (ci, r) in miss:
        c[ci][r] = None
    return c


def dump2(tag, idx, nc, nr, num):
    s = "page.active = %d;\n#DUMP %s %d %d\n" % (idx, tag, idx, nr)
    for c in range(1, nc + 1):
        for r in range(1, nr + 1):
            if num:
                s += "double dv = wcol(%d)[%d];\n?dv\n" % (c, r)
            s += "string ds$ = wcol(%d)[%d]$;\n$ds$\n" % (c, r)
    return s


c1 = mk(20)
c2 = mk(20, [(0, 3), (1, 7), (2, 11), (3, 15), (1, 16)])
c3 = mk(8)
out = ""


def run(name, cols_, irng, ctrl, extra=""):
    global out
    out += cols(cols_)
    out += "pcorrcoef irng:=(%s) control:=(%s) %s rt:=<new name:=R%s> corr:=<new name:=C%s>;\n" % (irng, ctrl, extra, name, name)
    out += dump2(name + "c", 3, 5, 12, True) + dump2(name + "r", 2, 20, 12, False)


c4 = mk(4)
c5 = mk(3)
cc = mk(20)
cc[2] = [1.0] * 20
cl = mk(20)
cl[3] = list(cl[2])
cp = mk(20)
cp[1] = [round(2 * v + 1, 3) for v in cp[0]]
run("h", c1, "col(1),col(2),col(3)", "col(4)", "tail:=two sig:=0 flag:=0")
run("i", c4, "col(1),col(2)", "col(3),col(4)", "tail:=two sig:=1")
run("j", c5, "col(1),col(2)", "col(3)", "tail:=two sig:=1")
run("k", cc, "col(1),col(2)", "col(3)", "tail:=two sig:=1")
run("l", cl, "col(1),col(2)", "col(3),col(4)", "tail:=two sig:=1")
run("m", cp, "col(1),col(2)", "col(3)", "tail:=two sig:=1")
run("n", c1, "col(1)", "col(3)", "tail:=two sig:=1")
run("o", c1, "col(1),col(2)", "col(1)", "tail:=two sig:=1")
open(sys.argv[1], 'w').write(out)

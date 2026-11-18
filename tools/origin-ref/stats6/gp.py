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
    out += dump2(name + "c", 3, 8, 40, True) + dump2(name + "r", 2, 30, 18, False)


run("a", c1, "col(1),col(2)", "col(3)", "tail:=two sig:=1 flag:=1")
run("b", c1, "col(1),col(2)", "col(3),col(4)", "tail:=two sig:=1 flag:=1")
run("c", c1, "col(1),col(2),col(3)", "col(4)", "tail:=one sig:=1 flag:=1")
run("d", c2, "col(1),col(2),col(3)", "col(4)", "tail:=two missing:=pairwise sig:=1 flag:=1")
run("e", c2, "col(1),col(2),col(3)", "col(4)", "tail:=two missing:=listwise sig:=1 flag:=1")
run("g", c3, "col(1),col(2)", "col(3),col(4)", "tail:=two sig:=1")
open(sys.argv[1], 'w').write(out)

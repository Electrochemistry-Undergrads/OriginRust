"""Third set of 2D binning reference runs (after g4.py): a number of intervals with the range left automatic, the bin
centres with only an increment, a Z column designated as Z, and the X-Function's own help line.
Same output format as g3.py, read with parse3.py. usage: python g5.py cases5.json script.lines.txt"""
import json, random, sys

random.seed(20261115)
rx = [round(random.uniform(-3, 17), 2) for _ in range(40)]
ry = [round(random.gauss(50, 12), 1) for _ in range(40)]
rz = [round(random.uniform(0, 100), 1) for _ in range(40)]
x0 = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 1.2, 1.7, 2.2, 8.1, 8.8, 10.0, 0.0]
y0 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 2.2, 2.7, 9.1, 0.4, 3.3, 5, 0]
z0 = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2]
x1 = [round(0.37 * i * i - 3.1, 3) for i in range(12)]
y1 = [round(5.5 + 2.3 * ((i * 7) % 11), 3) for i in range(12)]
cases = []
def add(name, x, y, z, opts, cols, pre=""):
    cases.append(dict(name=name, x=x, y=y, z=z, opts=opts, cols=cols, pre=pre))
for nm, o in [('i_auto', "x.intervals:=4 x.stepby:=1 y.intervals:=3 y.stepby:=1"),
              ('i_auto5', "x.intervals:=5 x.stepby:=1 y.intervals:=6 y.stepby:=1"),
              ('i_min', "x.min:=0 x.intervals:=4 x.stepby:=1 y.min:=20 y.intervals:=3 y.stepby:=1"),
              ('i_max', "x.max:=20 x.intervals:=4 x.stepby:=1 y.max:=80 y.intervals:=3 y.stepby:=1"),
              ('c_inc', "x.inc:=4 x.bin:=0 y.inc:=10 y.bin:=0"),
              ('c_mininc', "x.min:=0 x.inc:=4 x.bin:=0 y.min:=20 y.inc:=10 y.bin:=0"),
              ('o_auto', "x.border.outleft:=1 x.border.outright:=1 y.border.outleft:=1 y.border.outright:=1")]:
    add(nm, rx, ry, None, "stats:=count " + o, "(1,2)")
add('i_auto_x1', x1, y1, None, "stats:=count x.intervals:=4 x.stepby:=1 y.intervals:=3 y.stepby:=1", "(1,2)")
add('auto_x1', x1, y1, None, "stats:=count", "(1,2)")
add('z_desig', x0, y0, z0, "stats:=mean x.min:=0 x.max:=10 x.inc:=2 y.min:=0 y.max:=10 y.inc:=5", "(1,2,3)", "wks.col3.type=6;")
add('z_rz', rx, ry, rz, "stats:=sum x.min:=0 x.max:=10 x.inc:=5 y.min:=20 y.max:=80 y.inc:=20", "(1,2,3)", "wks.col3.type=6;")
add('z_iz', x0, y0, z0, "stats:=mean x.min:=0 x.max:=10 x.inc:=2 y.min:=0 y.max:=10 y.inc:=5 iz:=col(3)", "(1,2)", "")
json.dump(cases, open(sys.argv[1], 'w'))

def dump(book, nc, nr, tag):
    s = f"#DUMP {tag} {book} {nr}\n"
    for c in range(1, nc + 1):
        for r in range(1, nr + 1):
            s += f"double dv = wcol({c})[{r}];\n?dv\nstring ds$ = wcol({c})[{r}]$;\n$ds$\n"
    return s

out = ""
for ci, c in enumerate(cases):
    out += "newbook;\n" + f"col(1) = {{{', '.join(repr(float(a)) for a in c['x'])}}};\n"
    out += f"wks.addcol(); col(2) = {{{', '.join(repr(float(a)) for a in c['y'])}}};\n"
    if c['z'] is not None:
        out += f"wks.addcol(); col(3) = {{{', '.join(repr(float(a)) for a in c['z'])}}};\n"
    if c['pre']:
        out += c['pre'] + "\n"
    out += "newbook;\n"
    out += f"twoDBinning iy:=[Book{2*ci+1}]Sheet1!{c['cols']} {c['opts']} col:=[Book{2*ci+2}]Sheet1!\n"
    out += f"#CASE {ci} {c['name']}\n?wks.ncols\n?wks.nrows\n"
    for k in range(1, 9):
        out += f"$wks.col{k}.lname$\n$wks.col{k}.comment$\n"
    out += dump(f"Book{2*ci+2}", 8, 13, c["name"])
open(sys.argv[2], 'w').write(out)

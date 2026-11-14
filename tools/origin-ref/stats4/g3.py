"""Writes the LabTalk script (lines format) of the 2D binning reference runs: the twoDBinning X-Function writing its
worksheet form (bin ends, one column per Y bin). The cells and the column labels are read one by one.
usage: python g3.py cases3.json script.lines.txt"""
import json, math, random, sys

random.seed(20261115)
cases = []
def add(name, xs, ys, zs, opts):
    cases.append(dict(name=name, x=xs, y=ys, z=zs, opts=opts))

x0 = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 1.2, 1.7, 2.2, 8.1, 8.8, 10.0, 0.0]
y0 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 2.2, 2.7, 9.1, 0.4, 3.3, 5, 0]
z0 = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2]
rx = [round(random.uniform(-3, 17), 2) for _ in range(40)]
ry = [round(random.gauss(50, 12), 1) for _ in range(40)]
rz = [round(random.uniform(0, 100), 1) for _ in range(40)]
base = "x.min:=0 x.max:=10 x.inc:=2 y.min:=0 y.max:=10 y.inc:=5"
for st in ('count', 'mean', 'sum', 'min', 'max', 'median', 'freq'):
    add(f'xy_{st}', x0, y0, None, f"stats:={st} {base}")
    add(f'xz_{st}', x0, y0, z0, f"stats:={st} {base}")
# bin specifications
for nm, o in [('inc3', "x.min:=0 x.max:=10 x.inc:=3 y.min:=0 y.max:=10 y.inc:=4"),
              ('inc3b', "x.min:=0 x.max:=10 x.inc:=3 x.stepby:=0 y.min:=0 y.max:=10 y.inc:=4 y.stepby:=0"),
              ('int4', "x.min:=0 x.max:=10 x.intervals:=4 x.stepby:=1 y.min:=0 y.max:=10 y.intervals:=3 y.stepby:=1"),
              ('int4b', "x.min:=0 x.max:=10 x.intervals:=4 y.min:=0 y.max:=10 y.intervals:=3"),
              ('center', "x.min:=0 x.max:=10 x.inc:=2 x.bin:=0 y.min:=0 y.max:=10 y.inc:=5 y.bin:=0"),
              ('end', "x.min:=0 x.max:=10 x.inc:=2 x.bin:=1 y.min:=0 y.max:=10 y.inc:=5 y.bin:=1"),
              ('outl', "x.min:=0 x.max:=10 x.inc:=2 x.border.outleft:=1 y.min:=0 y.max:=10 y.inc:=5 y.border.outleft:=1"),
              ('outr', "x.min:=0 x.max:=10 x.inc:=2 x.border.outright:=1 y.min:=0 y.max:=10 y.inc:=5 y.border.outright:=1"),
              ('outlr', "x.min:=0 x.max:=10 x.inc:=2 x.border.outleft:=1 x.border.outright:=1 y.min:=0 y.max:=10 y.inc:=5 y.border.outleft:=1 y.border.outright:=1"),
              ('auto', ""),
              ('auto_inc', "x.inc:=2 y.inc:=5"),
              ('auto_int', "x.intervals:=5 y.intervals:=4"),
              ('minonly', "x.min:=2 y.min:=20"),
              ('maxonly', "x.max:=12 y.max:=60")]:
    add(f'r_{nm}', rx, ry, None, "stats:=count " + o)
    add(f'rm_{nm}', rx, ry, None, "stats:=mean " + o)
add('r_negmin', rx, ry, None, "stats:=count x.min:=-3 x.max:=17 x.inc:=4 y.min:=20 y.max:=80 y.inc:=15")
add('r_fine', rx, ry, None, "stats:=count x.min:=-3 x.max:=17 x.inc:=1 y.min:=20 y.max:=80 y.inc:=10")
add('r_period', rx, ry, None, "stats:=count x.min:=0 x.max:=10 x.inc:=2 x.periodical:=1 x.period:=20 y.min:=20 y.max:=80 y.inc:=15")
json.dump(cases, open(sys.argv[1], 'w'))

def dump(book, nc, nr, tag):
    s = f"#DUMP {tag} {book} {nr}\n"
    for c in range(1, nc + 1):
        for r in range(1, nr + 1):
            s += f"double dv = wcol({c})[{r}];\n?dv\nstring ds$ = wcol({c})[{r}]$;\n$ds$\n"
    return s

out = ""
for ci, c in enumerate(cases):
    def col(i, v):
        return ("wks.addcol(); " if i else "") + f"col({i+1}) = {{{', '.join(repr(float(a)) for a in v)}}};\n"
    out += "newbook;\n" + col(0, c['x']) + col(1, c['y'])
    if c['z'] is not None:
        out += col(2, c['z'])
    cols = "(1,3)" if c['z'] is not None else "(1,2)"
    out += f"newbook;\n"
    out += f"twoDBinning iy:=[Book{2*ci+1}]Sheet1!{cols} {c['opts']} col:=[Book{2*ci+2}]Sheet1!\n"
    out += f"#CASE {ci} {c['name']}\n?wks.ncols\n?wks.nrows\n"
    for k in range(1, 11):
        out += f"$wks.col{k}.lname$\n$wks.col{k}.comment$\n"
    out += dump(f"Book{2*ci+2}", 10, 22, c["name"])
open(sys.argv[2], 'w').write(out)

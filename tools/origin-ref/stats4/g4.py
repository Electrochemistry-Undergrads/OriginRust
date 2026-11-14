"""Second set of 2D binning reference runs (after g3.py): three-column (X, Y, Z) input, a long name on the X column, and
missing values. Same output format as g3.py, read with parse3.py.
usage: python g4.py cases4.json script.lines.txt"""
import json, sys

x0 = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 1.2, 1.7, 2.2, 8.1, 8.8, 10.0, 0.0]
y0 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 2.2, 2.7, 9.1, 0.4, 3.3, 5, 0]
z0 = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2]
base = "x.min:=0 x.max:=10 x.inc:=2 y.min:=0 y.max:=10 y.inc:=5"
cases = []
for st in ('count', 'mean', 'sum', 'min', 'max', 'median', 'freq'):
    cases.append(dict(name=f'xyz_{st}', x=x0, y=y0, z=z0, opts=f"stats:={st} {base}", cols="(1,2,3)", pre=""))
cases.append(dict(name='lname', x=x0, y=y0, z=None, opts="stats:=mean " + base, cols="(1,2)",
                  pre='col(1)[L]$="Potential"; col(1)[U]$="V"; col(2)[L]$="Current"; col(2)[U]$="A";'))
cases.append(dict(name='lname_z', x=x0, y=y0, z=z0, opts="stats:=mean " + base, cols="(1,2,3)",
                  pre='col(1)[L]$="Potential"; col(2)[L]$="Current"; col(3)[L]$="Charge"; col(3)[U]$="C";'))
xm = list(x0); xm[2] = None
ym = list(y0); ym[3] = None
zm = list(z0); zm[4] = None
cases.append(dict(name='missing', x=xm, y=ym, z=zm, opts="stats:=count " + base, cols="(1,2,3)", pre=""))
cases.append(dict(name='missing_mean', x=xm, y=ym, z=zm, opts="stats:=mean " + base, cols="(1,2,3)", pre=""))
cases.append(dict(name='xyz_auto', x=x0, y=y0, z=z0, opts="stats:=count", cols="(1,2,3)", pre=""))
for nm, o in [('c_center', "x.min:=0 x.max:=10 x.inc:=2 x.bin:=0 y.min:=0 y.max:=10 y.inc:=5 y.bin:=0"),
              ('c_center_int', "x.min:=0 x.max:=10 x.intervals:=4 x.stepby:=1 x.bin:=0 y.min:=0 y.max:=10 y.intervals:=2 y.stepby:=1 y.bin:=0"),
              ('c_center_auto', "x.bin:=0 y.bin:=0"),
              ('c_auto_inc', "x.inc:=2 y.inc:=5"),
              ('c_auto_inc3', "x.inc:=3 y.inc:=3"),
              ('c_auto_int', "x.intervals:=4 x.stepby:=1 y.intervals:=3 y.stepby:=1"),
              ('c_min_inc', "x.min:=1 x.inc:=2 y.min:=1 y.inc:=4"),
              ('c_max_inc', "x.max:=9 x.inc:=2 y.max:=9 y.inc:=4"),
              ('c_noninteger', "x.min:=0 x.max:=10 x.inc:=3 y.min:=0 y.max:=11 y.inc:=4")]:
    cases.append(dict(name=nm, x=x0, y=y0, z=None, opts="stats:=count " + o, cols="(1,2)", pre=""))
json.dump(cases, open(sys.argv[1], 'w'))

def dump(book, nc, nr, tag):
    s = f"#DUMP {tag} {book} {nr}\n"
    for c in range(1, nc + 1):
        for r in range(1, nr + 1):
            s += f"double dv = wcol({c})[{r}];\n?dv\nstring ds$ = wcol({c})[{r}]$;\n$ds$\n"
    return s

def lit(v):
    return ', '.join('0/0' if a is None else repr(float(a)) for a in v)

out = ""
for ci, c in enumerate(cases):
    out += "newbook;\n" + f"col(1) = {{{lit(c['x'])}}};\n" + f"wks.addcol(); col(2) = {{{lit(c['y'])}}};\n"
    if c['z'] is not None:
        out += f"wks.addcol(); col(3) = {{{lit(c['z'])}}};\n"
    if c['pre']:
        out += c['pre'] + "\n"
    out += "newbook;\n"
    out += f"twoDBinning iy:=[Book{2*ci+1}]Sheet1!{c['cols']} {c['opts']} col:=[Book{2*ci+2}]Sheet1!\n"
    out += f"#CASE {ci} {c['name']}\n?wks.ncols\n?wks.nrows\n"
    for k in range(1, 9):
        out += f"$wks.col{k}.lname$\n$wks.col{k}.comment$\n$wks.col{k}.unit$\n"
    out += dump(f"Book{2*ci+2}", 8, 14, c["name"])
open(sys.argv[2], 'w').write(out)

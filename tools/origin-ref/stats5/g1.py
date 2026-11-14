"""Writes the LabTalk line script of the Discrete Frequency (discfreqs) reference runs: text and number columns, missing
values, ties of counts, every sort order, the relative and cumulative columns as fractions or percents, case
sensitivity. The output sheet is read cell by cell (labels and text as shown).
usage: python g1.py cases1.json script.lines.txt"""
import json, sys

T = ["b", "a", "B", "c", "a", "b", "A", "", "d", "b", "a", "c", "Cc", "c"]
N = [3, 1, 2, 3, 3, 1.5, 2, None, 7, 1, 3, 2, 10, -1, 2]
T2 = ["x", "y", "x", "z", "y", "x"]
cases = []
def add(name, kind, data, opts, data2=None):
    cases.append(dict(name=name, kind=kind, data=data, data2=data2, opts=opts))
add('t_default', 'text', T, "")
add('t_all', 'text', T, "perc:=1 cumul:=1")
add('t_pct', 'text', T, "perc:=1 cumul:=1 show:=percent")
add('t_case', 'text', T, "sensitive:=1 perc:=1 cumul:=1")
for s in ('casc', 'cdesc', 'vasc', 'vdesc', 'cat'):
    add(f't_{s}', 'text', T, f"sort:={s} perc:=1 cumul:=1")
    add(f'n_{s}', 'num', N, f"sort:={s} perc:=1 cumul:=1")
add('n_nofreq', 'num', N, "freq:=0 perc:=1")
add('t_small', 'text', T2, "perc:=1 cumul:=1 show:=percent")
add('idx', 'idx', ["x", "y", "x", "z"], "type:=0 perc:=1 cumul:=1", [1, 2, 3, 4])
json.dump(cases, open(sys.argv[1], 'w'))

def lit_num(v):
    return ', '.join('0/0' if a is None else repr(float(a)) for a in v)

out = ""
for ci, c in enumerate(cases):
    out += "newbook;\n"
    if c['kind'] in ('text', 'idx'):
        out += "wks.col1.format=2;\n"
        out += ''.join(f'col(1)[{i+1}]$="{v}";\n' for i, v in enumerate(c['data']))
    else:
        out += f"col(1) = {{{lit_num(c['data'])}}};\n"
    rng = "col(1)"
    if c['data2'] is not None:
        out += f"wks.addcol(); col(2) = {{{lit_num(c['data2'])}}};\n"
        rng = "(col(1),col(2))"
    out += f"discfreqs irng:={rng} {c['opts']};\n"
    out += f"#CASE {ci} {c['name']}\n$page.name$\n$wks.name$\n?wks.ncols\n"
    for k in range(1, 7):
        out += f"$wks.col{k}.lname$\n$wks.col{k}.unit$\n$wks.col{k}.comment$\n?wks.col{k}.type\n?wks.col{k}.format\n"
    out += f"#DUMP {c['name']} 7 16\n"
    for k in range(1, 7):
        for r in range(1, 17):
            out += f"string ds$ = wcol({k})[{r}]$;\n$ds$\n"
open(sys.argv[2], 'w').write(out)

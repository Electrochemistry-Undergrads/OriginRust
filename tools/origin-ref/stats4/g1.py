"""Writes the LabTalk script (lines format) of the Holm reference runs: one-way means comparisons with the
Bonferroni-Holm (7) and Sidak-Holm (8) methods of Origin's library, on inputs generated here.
usage: python g1.py cases1.json script.lines.txt"""
import json, math, random, sys

random.seed(20261113)

def grp(n, mu, sd, nd=3):
    return [round(random.gauss(mu, sd), nd) for _ in range(n)]

cases = []
def add(name, groups, alpha=0.05, methods=(7, 8)):
    cases.append(dict(name=name, groups=groups, alpha=alpha, methods=list(methods)))

add('fixed4', [[10.5322, 9.9412, 11.5742, 9.9269, 12.5506, 9.998, 13.4233, 10.1888],
               [9.0823, 13.0082, 10.1355, 12.8924, 11.3015, 12.6897, 12.527, 12.4559, 13.7544, 12.2495],
               [10.1239, 13.8808, 10.55, 14.3831, 11.1195, 14.7433, 11.8268],
               [12.0353, 13.8067, 11.9695, 14.8306, 11.9978, 15.7406, 12.1577, 16.504, 12.4763]])
for k in (2, 3, 4, 5, 6):
    for rep in range(3):
        sizes = [random.randint(3, 9) for _ in range(k)]
        sd = random.choice([0.8, 1.5, 3.0])
        gs = [grp(sizes[i], 10 + random.uniform(0, 2.5 * (i if rep else 0.3 * i)), sd) for i in range(k)]
        add(f'r{k}_{rep}', gs)
add('bal5_small', [grp(5, 10 + 0.9 * i, 1.0) for i in range(5)])
add('bal5_a01', [grp(5, 10 + 0.9 * i, 1.0) for i in range(5)], alpha=0.01)
add('bal5_a10', [grp(5, 10 + 0.9 * i, 1.0) for i in range(5)], alpha=0.10)
add('one_obs', [[5.0], [6.1, 6.5, 5.9, 6.3], [7.2, 7.0, 7.9, 7.4, 7.7]])
add('tied_means', [[1.0, 2.0, 3.0], [1.0, 2.0, 3.0], [2.0, 3.0, 4.0], [1.0, 2.0, 3.0]])
add('const', [[5.0, 5.0, 5.0], [5.0, 5.0, 5.0, 5.0], [6.0, 6.0, 6.0]])
add('missing', [[1.5, None, 3.5, 4.5, 5.5, 6.5, 2.0], [1.0, 2.0, None, 5.0, 4.0, 7.5, 2.5, 8.0], [3.0, 9.0, 4.0, 6.0, 7.0]])
add('big_k8', [grp(6, 10 + 0.6 * i, 1.2) for i in range(8)])
# inputs found by a search for the case where the step-down rule and the plain comparison with alpha/(m-i+1) differ
for i, gs in enumerate(json.load(open('stepdown_groups.json'))):
    add(f'stepdown{i}', gs)
json.dump(cases, open(sys.argv[1], 'w'))

out = ""
for ci, c in enumerate(cases):
    out += "newbook;\n"
    if ci == 0:
        pass
    out += "int rc = run.LoadOC(%WORK%stats4/stats4-ref.c, 16);\n" if ci == 0 else ""
    for i, g in enumerate(c['groups']):
        vals = ', '.join('0' if v is None else repr(float(v)) for v in g)
        out += ("wks.addcol(); " if i else "") + f"col({i+1}) = {{{vals}}};\n"
        for r, v in enumerate(g):
            if v is None:
                out += f"col({i+1})[{r+1}] = 0/0;\n"
    k = len(c['groups'])
    for m in c['methods']:
        out += f"double r{ci}_{m} = sheaf_mc({k}, {m}, {c['alpha']});\n?r{ci}_{m}\n#[Book{ci+1}]Sheet1\n"
open(sys.argv[2], 'w').write(out)

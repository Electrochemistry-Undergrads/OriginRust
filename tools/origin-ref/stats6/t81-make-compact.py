"""Writes t81-origin.compact.json: the data and the numbers Origin 2016 gave in the T81 runs (partial
correlation, three-way ANOVA, repeated measures ANOVA). Reads the raw outputs of the runs, which are kept
in the git-ignored scratch folder (re-workspace/scratch/t81/), and the generators gp.py / g3.py.
usage: python t81-make-compact.py SCRATCH_DIR OUT.json"""
import json, re, sys, random
from pp import parse

scratch, out = sys.argv[1].rstrip('/') + '/', sys.argv[2]
res = {}

# ---- partial correlation (gp.py: seed 11) ----
sys.argv = ['x', '/dev/null']
src = open('gp.py').read().split('c1 = mk(20)')[0]
ns = {}
exec(src, ns)
random.seed(11)
c1 = ns['mk'](20); c2 = ns['mk'](20, [(0, 3), (1, 7), (2, 11), (3, 15), (1, 16)]); c3 = ns['mk'](8)
cases = {'a': (c1, [0, 1], [2], 'pairwise', 'two'), 'b': (c1, [0, 1], [2, 3], 'pairwise', 'two'),
         'c': (c1, [0, 1, 2], [3], 'pairwise', 'one'), 'd': (c2, [0, 1, 2], [3], 'pairwise', 'two'),
         'e': (c2, [0, 1, 2], [3], 'listwise', 'two'), 'g': (c3, [0, 1], [2, 3], 'pairwise', 'two')}
pc = {}
dumps = {d['tag']: d for d in parse(scratch + 'gp.raw.txt')}
for k, (cols, vi, ci, miss, tail) in cases.items():
    ra = dumps[k + 'r']['cells']; nv = len(vi); nc = len(ci)
    desc = [[ra.get((c, r)) for c in range(9, 15)] for r in range(1, nv + nc + 1)]
    r = [[None] * nv for _ in range(nv)]; p = [[None] * nv for _ in range(nv)]; df = [[None] * nv for _ in range(nv)]
    for i in range(nv):
        for j in range(nv):
            col = 15 + j
            r[i][j] = ra.get((col, 3 * i + 1)); p[i][j] = ra.get((col, 3 * i + 2)); df[i][j] = ra.get((col, 3 * i + 3))
    pc[k] = dict(data=cols, vars=vi, controls=ci, missing=miss, tail=tail, desc_n_mean_sd_sum_min_max=desc, r=r, p=p, df=df)
res['pcorrcoef'] = pc

# ---- three-way ANOVA (g3.py) ----
sys.path.insert(0, '.')
import g3
a3 = {}
for name, cells, raw in (('bal', g3.BAL, scratch + 'a3-bal.raw.txt'), ('unb', g3.UNB, scratch + 'a3-unb.raw.txt')):
    cells_ = {}
    for l in open(raw, encoding='utf-8', errors='replace'):
        m = re.match(r'wcol\((\d+)\)\[(\d+)\]\$ = (.*)', l.rstrip('\r\n'))
        if m and m.group(3).strip(): cells_[(int(m.group(1)), int(m.group(2)))] = m.group(3)
    cs = (62, 63, 64, 65, 66, 67, 68, 69) if name == 'unb' else (62, 63, 64, 65, 66, 67, 68, 69)
    tab = []
    labels = ['A', 'B', 'C', 'A*B', 'A*C', 'B*C', 'A*B*C', 'Model', 'Error', 'Total']
    for i, lab in enumerate(labels):
        row = [cells_.get((c, i + 1)) for c in (64, 65, 66, 67, 68)]
        tab.append(dict(term=lab, df=row[0], ss=row[1], ms=row[2], f=row[3], p=row[4]))
    a3[name] = dict(rows=[list(r) for r in g3.three(cells)], table=tab)
res['anova3'] = a3

# ---- one-way repeated measures (indexed A level, B data, C subject) ----
tab = g3.rm_data(8, 4)
cells_ = {}
for l in open(scratch + 'rm1-i.raw.txt', encoding='utf-8', errors='replace'):
    m = re.match(r'wcol\((\d+)\)\[(\d+)\]\$ = (.*)', l.rstrip('\r\n'))
    if m and m.group(3).strip(): cells_[(int(m.group(1)), int(m.group(2)))] = m.group(3)
g = lambda c, r: cells_.get((c, r))
res['rmanova1'] = dict(
    table=tab,
    multivariate_cols6_10=[[g(c, r) for c in range(6, 11)] for r in range(1, 5)],
    mauchly=dict(w=g(11, 1), chisq=g(12, 1), df=g(13, 1), p=g(14, 1)),
    epsilon=dict(greenhouse_geisser=g(15, 1), huynh_feldt=g(16, 1), lower_bound=g(17, 1)),
    within=[dict(source=s, ss=g(18, i + 1), df=g(19, i + 1), ms=g(20, i + 1), f=g(21, i + 1), p=g(22, i + 1)) for i, s in enumerate(['sphericity assumed', 'greenhouse-geisser', 'huynh-feldt', 'lower-bound'])],
    error=[dict(source=s, ss=g(18, i + 5), df=g(19, i + 5), ms=g(20, i + 5)) for i, s in enumerate(['sphericity assumed', 'greenhouse-geisser', 'huynh-feldt', 'lower-bound'])],
    intercept=dict(ss=g(23, 1), df=g(24, 1), ms=g(25, 1), f=g(26, 1), p=g(27, 1)),
    subjects=dict(ss=g(23, 2), df=g(24, 2), ms=g(25, 2)),
    level_means=[dict(mean=g(28, r), se=g(29, r), lcl=g(30, r), ucl=g(31, r)) for r in range(1, 5)],
    tukey_pairs=[dict(level=g(33, r), diff=g(34, r), se=g(35, r), df=g(36, r), stat=g(37, r), p=g(38, r), sig=g(40, r)) for r in range(1, 7)],
    note='means comparison blocks of 10 columns from column 33: Tukey, Bonferroni, Dunn-Sidak, Fisher LSD, Scheffe, then Holm-Bonferroni and Holm-Sidak (8 columns each, sorted by p)')
# ---- two-way repeated measures ----
rows = json.load(open(scratch + 'rm2-rows.json'))
cells_ = {}
for l in open(scratch + 'rm2.raw.txt', encoding='utf-8', errors='replace'):
    m = re.match(r'wcol\((\d+)\)\[(\d+)\]\$ = (.*)', l.rstrip('\r\n'))
    if m and m.group(3).strip(): cells_[(int(m.group(1)), int(m.group(2)))] = m.group(3)
g = lambda c, r: cells_.get((c, r))
blocks = {}
for name, r0 in (('A', 1), ('B', 9), ('AB', 17)):
    blocks[name] = dict(
        effect=[dict(ss=g(18, r0 + i), df=g(19, r0 + i), ms=g(20, r0 + i), f=g(21, r0 + i), p=g(22, r0 + i)) for i in range(4)],
        error=[dict(ss=g(18, r0 + 4 + i), df=g(19, r0 + 4 + i), ms=g(20, r0 + 4 + i)) for i in range(4)])
res['rmanova2'] = dict(rows_A_B_data_subject=rows, blocks=blocks,
    mauchly_B=dict(w=g(11, 2), chisq=g(12, 2), df=g(13, 2), p=g(14, 2), gg=g(15, 2), hf=g(16, 2), lb=g(17, 2)),
    mauchly_AB=dict(w=g(11, 3), chisq=g(12, 3), df=g(13, 3), p=g(14, 3), gg=g(15, 3), hf=g(16, 3), lb=g(17, 3)),
    subjects=dict(ss=g(23, 2), df=g(24, 2), ms=g(25, 2)), intercept=dict(ss=g(23, 1), df=g(24, 1), ms=g(25, 1), f=g(26, 1), p=g(27, 1)))
json.dump(res, open(out, 'w'), indent=1)
print('wrote', out)

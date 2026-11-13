from gen import *
import math, json, sys
from g2 import seq, a, b, c, ka, kb


def signpair(n, npos):
    signs = [1] * npos + [-1] * (n - npos)
    order = sorted(range(n), key=lambda i: (i * 37) % n)
    sg = [0] * n
    for k, i in enumerate(order):
        sg[i] = signs[k]
    x = []; y = []
    for i in range(n):
        d = round((i + 1) * 0.1 + 0.03 * ((i * 7) % 5), 3)
        base = 10 + (i * 13 % 9)
        x.append(round(base + (d if sg[i] > 0 else 0), 3))
        y.append(round(base + (d if sg[i] < 0 else 0), 3))
    return x, y


cases = []
for n, npos in ((10, 7), (12, 9), (15, 11), (18, 13), (20, 14), (22, 16), (25, 18), (29, 21), (40, 29)):
    x, y = signpair(n, npos); cases.append(('sign', f'sn{n}', [x, y], dict(tail='two')))
x, y = signpair(30, 19)
for t in ('two', 'upper', 'lower'):
    cases.append(('sign', f'sw30_{t}', [y, x], dict(tail=t)))
    cases.append(('sign', f'sw30b_{t}', [x, y], dict(tail=t)))
x, y = signpair(8, 6)
for t in ('two', 'upper', 'lower'):
    cases.append(('sign', f'sw8_{t}', [y, x], dict(tail=t)))
    cases.append(('sign', f'sw8b_{t}', [x, y], dict(tail=t)))
x, y = signpair(40, 28)
cases.append(('sign', 'sn40ties', [x + [5.0] * 6, y + [5.0] * 6], dict(tail='two')))
# KS
cases.append(('ks', 'ksw_two', [kb, ka], dict(tail='two')))
cases.append(('ks', 'ksw_up', [kb, ka], dict(tail='upper')))
cases.append(('ks', 'ksw_low', [kb, ka], dict(tail='lower')))
cases.append(('ks', 'k10_n100', [seq(100, 1.1, 20, 1), seq(120, 0.8, 20, 1, 1.0)], dict(tail='two')))
cases.append(('ks', 'k11_n300', [seq(300, 1.1, 20, 1), seq(400, 0.8, 20, 1, 0.6)], dict(tail='two')))
for t in ('two', 'upper', 'lower'):
    cases.append(('ks', f'k12_n35{t}', [seq(35, 1.3, 20, 1), seq(40, 0.6, 20, 1, 1.0)], dict(tail=t)))
    cases.append(('ks', f'k13_n10{t}', [seq(10, 1.3, 20, 3), seq(11, 0.6, 20, 3, 1.0)], dict(tail=t)))
    cases.append(('ks', f'k14_ties{t}', [[1, 1, 1, 2, 2, 3, 3, 3, 3, 4], [1, 2, 2, 2, 3, 4, 4, 4, 5, 5, 5]], dict(tail=t)))
# Friedman, with the p-value column
for nm, cs in (('f1_3', [a, b, c]),
               ('f2_4', [seq(7, 1.3, 10, 0), seq(7, 0.9, 10, 0, 1.0), seq(7, 2.3, 10, 0, 2.0), seq(7, 1.7, 10, 0, 0.5)]),
               ('f4_k2', [a, b]), ('f5_const', [[5.0] * 4] * 3),
               ('f6_big', [seq(25, 1.2, 20, 2), seq(25, 0.8, 20, 2, 2.0), seq(25, 2.2, 20, 2, 1.0), seq(25, 1.5, 20, 2, 0.0), seq(25, 0.5, 20, 2, 3.0)]),
               ('f7_ordered', [[1.0, 2.0, 3.0, 4.0], [2.0, 3.0, 4.0, 5.0], [3.0, 4.0, 5.0, 6.0]])):
    cases.append(('fr', nm + 'p', cs, dict(tail='two')))
cases.append(('fr', 'f8_tied_rows', [[1.0, 2.0, 2.0, 5.0, 3.0, 3.0], [1.0, 2.0, 3.0, 4.0, 3.0, 1.0], [2.0, 2.0, 3.0, 1.0, 3.0, 2.0], [1.0, 5.0, 3.0, 1.0, 3.0, 2.0]], dict(tail='two')))
cases.append(('fr', 'f9_k5_n3', [[1.0, 2.0, 3.0], [2.0, 1.0, 5.0], [3.0, 4.0, 4.0], [5.0, 3.0, 1.0], [4.0, 5.0, 2.0]], dict(tail='two')))
cases.append(('fr', 'f10_n2', [[1.0, 2.0], [2.0, 1.0], [3.0, 4.0]], dict(tail='two')))
cases.append(('fr', 'f11_unequal', [[1.0, 2.0, 3.0, 4.0], [2.0, 3.0, 4.0], [3.0, 4.0, 5.0, 6.0]], dict(tail='two')))
# median test
cases.append(('med', 'm8_tiemed', [[1.0, 2.0, 3.0, 4.0, 5.0], [3.0, 3.0, 6.0, 7.0], [3.0, 8.0, 9.0, 10.0, 11.0, 2.0]], dict(tail='two')))
cases.append(('med', 'm9_k2_small', [[1.0, 2.0], [3.0, 4.0]], dict(tail='two')))
cases.append(('med', 'm10_zero_cell', [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]], dict(tail='two')))
cases.append(('med', 'm11_n1', [[1.0], [2.0, 3.0]], dict(tail='two')))
# qtest
q6 = [10.0, 10.2, 10.1, 9.9, 10.05, 12.0]
for nm, al in (('qa_n6', 0.05), ('qb_n6', 0.1), ('qc_n6', 0.01)):
    cases.append(('qt', nm, [q6], dict(alpha=al)))
cases.append(('qt', 'qd_tie', [[1.0, 2.0, 3.0]], dict(alpha=0.05)))
cases.append(('qt', 'qe_tie', [[1.0, 2.0, 2.0, 3.0]], dict(alpha=0.05)))
cases.append(('qt', 'qf_n10', [[1.0, 1.1, 1.2, 1.15, 1.05, 1.12, 1.18, 1.08, 1.14, 2.0]], dict(alpha=0.05)))
q9 = [1.0, 1.1, 1.2, 1.15, 1.05, 1.12, 1.18, 1.08, 1.14]
cases.append(('qt', 'qg_n9', [q9], dict(alpha=0.1)))
cases.append(('qt', 'qg_n9b', [q9], dict(alpha=0.01)))
q5 = [1.0, 1.1, 1.2, 1.15, 3.0]
for nm, al in (('qh_n5', 0.1), ('qh_n5b', 0.01), ('qh_n5c', 0.05)):
    cases.append(('qt', nm, [q5], dict(alpha=al)))
q7 = [1.0, 1.1, 1.2, 1.15, 1.17, 1.12, 3.0]
for nm, al in (('qi_n7', 0.1), ('qi_n7b', 0.01), ('qi_n7c', 0.05)):
    cases.append(('qt', nm, [q7], dict(alpha=al)))
q8 = [10.0, 10.1, 9.9, 10.2, 10.05, 9.95, 10.15, 11.0]
for nm, al in (('qj_n8b', 0.1), ('qj_n8c', 0.01)):
    cases.append(('qt', nm, [q8], dict(alpha=al)))
# grubbs extra
cases.append(('gr', 'gx_n5', [[1.0, 1.1, 1.2, 1.15, 3.0]], dict(alpha=0.05)))
cases.append(('gr', 'gx_big', [seq(200, 1.1, 10, 2) + [40.0]], dict(alpha=0.05)))
cases.append(('gr', 'gx_equal_ends', [[1.0, 2.0, 3.0, 4.0, 5.0]], dict(alpha=0.05)))


def script():
    out = ""
    for kind, nm, cs, kw in cases:
        out += cols(cs); nc = len(cs); rng = ",".join(str(i + 1) for i in range(nc)); al = kw.get('alpha', 0.05); sh = 'R' + nm
        if kind == 'sign':
            out += f"sign2 irng:=({rng}) null:=0 tail:={kw['tail']} alpha:={al} rt:=<new name:={sh}>;\n" + dump("%H", sh, 16, 5, nm)
        elif kind == 'ks':
            out += f"kstest2 type:=1 irng:=({rng}) tail:={kw['tail']} alpha:={al} rt:=<new name:={sh}>;\n" + dump("%H", sh, 18, 5, nm)
        elif kind == 'med':
            out += f"mediantest type:=1 irng:=({rng}) alpha:={al} rt:=<new name:={sh}>;\n" + dump("%H", sh, 18, 6, nm)
        elif kind == 'fr':
            out += f"friedman type:=1 irng:=({rng}) alpha:={al} rt:=<new name:={sh}>;\n" + dump("%H", sh, 18, 7, nm)
        elif kind == 'gr':
            out += 'double a, c, d, e; int b, f; string g$; a=0; b=0; c=0; d=0; e=0; f=0; g$="";\n'
            out += f"grubbs ix:=col(A) alpha:={al} ox:=a index:=b gstat:=c critical:=d pval:=e sig:=f conclusion:=g$ box:=0;\n#VARS {nm}\n?a\n?b\n?c\n?d\n?e\n?f\n$g$\n"
        elif kind == 'qt':
            out += 'double a, c, d; int b, f; string g$; a=0; b=0; c=0; d=0; f=0; g$="";\n'
            out += f"qtest ix:=col(A) alpha:={al} ox:=a index:=b qstat:=c critical:=d sig:=f conclusion:=g$ box:=0;\n#VARS {nm}\n?a\n?b\n?c\n?d\n?f\n$g$\n"
    return out


if __name__ == '__main__':
    json.dump([dict(kind=k, name=n, cols=cs, **kw) for k, n, cs, kw in cases], open('cases3.json', 'w'))
    open(sys.argv[1], 'w').write(script())

from gen import *
import math, json, sys

def seq(n, seed, scale=10.0, nd=1, shift=0.0):
    return [round(shift + scale*(0.5+0.5*math.sin(seed*i + 0.3*seed*i*i/(i+3.0))), nd) for i in range(1, n+1)]

cases = []

def add(kind, name, cols_, **kw):
    cases.append(dict(kind=kind, name=name, cols=cols_, **kw))

a = [1.1, 2.3, 2.3, 4.0, 5.5, 6.1, 7.2, 7.2, 9.9]
b = [2.2, 3.1, 4.0, 4.0, 8.8, 10.5, 11.0, 12.5, 12.9]
c = [3.3, 1.1, 5.0, 2.2, 8.0, 7.5, 1.0, 9.1, 4.4]
# sign2
for tail in ('two', 'upper', 'lower'):
    add('sign', 's1_' + tail, [a, b], tail=tail, null=0)
add('sign', 's2_null', [a, b], tail='two', null=0.5)
add('sign', 's2b_null', [a, c], tail='two', null=1.0)
p1 = seq(30, 1.9, 10, 2); p2 = seq(30, 0.7, 10, 2, 0.5)
add('sign', 's3_n30', [p1, p2], tail='two', null=0)
add('sign', 's3_n30u', [p1, p2], tail='upper', null=0)
p3 = seq(70, 1.3, 10, 2); p4 = seq(70, 2.9, 10, 2, 1.0)
add('sign', 's4_n70', [p3, p4], tail='two', null=0)
add('sign', 's4_n70l', [p3, p4], tail='lower', null=0)
add('sign', 's5_missing', [[1.5, None, 3.5, 4.5, 5.5, 6.5, 2.0], [1.0, 2.0, None, 5.0, 4.0, 7.5, 2.0]], tail='two', null=0)
add('sign', 's6_same', [[1.0, 2.0, 3.0, 4.0, 5.0], [1.0, 2.0, 3.0, 4.0, 5.0]], tail='two', null=0)
add('sign', 's7_n1', [[2.0], [1.0]], tail='two', null=0)
add('sign', 's8_bal', [[1.0, 3.0, 5.0, 7.0], [2.0, 2.0, 6.0, 6.0]], tail='two', null=0)
add('sign', 's9_alpha', [a, b], tail='two', null=0, alpha=0.1)
# kstest2
ka = [1.1, 2.3, 2.3, 4.0, 5.5, 6.1, 7.2, 7.2, 9.9]; kb = [2.2, 3.1, 4.0, 4.0, 8.8, 10.5, 11.0]
for tail in ('two', 'upper', 'lower'):
    add('ks', 'k1_' + tail, [ka, kb], tail=tail)
k2a = [1.2, 3.4, 2.2, 9.1, 5.7, 8.3]; k2b = [4.1, 6.6, 7.3, 10.2, 12.0, 11.4, 5.0, 13.3]
for tail in ('two', 'upper', 'lower'):
    add('ks', 'k2_' + tail, [k2a, k2b], tail=tail)
add('ks', 'k3_n25', [seq(25, 2.1, 20, 1), seq(30, 1.7, 20, 1, 2.0)], tail='two')
add('ks', 'k3_n25u', [seq(25, 2.1, 20, 1), seq(30, 1.7, 20, 1, 2.0)], tail='upper')
add('ks', 'k4_n60', [seq(60, 1.1, 20, 2), seq(75, 0.8, 20, 2, 3.0)], tail='two')
add('ks', 'k5_missing', [[1.5, None, 3.5, 4.5, 5.5, 6.5, 2.0], [1.0, 2.0, None, 5.0, 4.0, 7.5, 2.5, 8.0]], tail='two')
add('ks', 'k6_const', [[5.0]*4, [5.0]*5], tail='two')
add('ks', 'k7_disjoint', [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0, 7.0]], tail='two')
add('ks', 'k8_one', [[3.5], [1.0, 2.0, 4.0, 5.0, 6.0]], tail='two')
add('ks', 'k9_alpha', [ka, kb], tail='two', alpha=0.1)
# median test
add('med', 'm1_3', [a, b, c])
add('med', 'm2_2', [a, b])
add('med', 'm3_4', [seq(12, 1.3, 10, 0), seq(15, 0.9, 10, 0, 1.0), seq(10, 2.3, 10, 0, 2.0), seq(9, 1.7, 10, 0, 0.5)])
add('med', 'm4_missing', [[1.5, None, 3.5, 4.5, 5.5, 6.5, 2.0], [1.0, 2.0, None, 5.0, 4.0, 7.5, 2.5, 8.0], [3.0, 9.0, 4.0, 6.0, 7.0]])
add('med', 'm5_const', [[5.0]*4, [5.0]*5])
add('med', 'm6_odd', [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0, 7.0]])
add('med', 'm7_big', [seq(40, 1.2, 20, 1), seq(50, 0.8, 20, 1, 2.0), seq(35, 2.2, 20, 1, 1.0)])
# friedman
add('fr', 'f1_3', [a, b, c])
add('fr', 'f2_4', [seq(7, 1.3, 10, 0), seq(7, 0.9, 10, 0, 1.0), seq(7, 2.3, 10, 0, 2.0), seq(7, 1.7, 10, 0, 0.5)])
add('fr', 'f3_missing', [[1.5, None, 3.5, 4.5, 5.5, 6.5, 2.0], [1.0, 2.0, 4.0, 5.0, 4.0, 7.5, 2.5], [3.0, 9.0, 4.0, 6.0, 7.0, 5.5, 1.0]])
add('fr', 'f4_k2', [a, b])
add('fr', 'f5_const', [[5.0]*4, [5.0]*4, [5.0]*4])
add('fr', 'f6_big', [seq(25, 1.2, 20, 2), seq(25, 0.8, 20, 2, 2.0), seq(25, 2.2, 20, 2, 1.0), seq(25, 1.5, 20, 2, 0.0), seq(25, 0.5, 20, 2, 3.0)])
add('fr', 'f7_ordered', [[1.0, 2.0, 3.0, 4.0], [2.0, 3.0, 4.0, 5.0], [3.0, 4.0, 5.0, 6.0]])
# grubbs
G = [('g1', a), ('g2_high', [2.1, 2.4, 2.2, 2.6, 2.3, 2.5, 9.8, 2.2]), ('g3_low', [-7.5, 2.4, 2.2, 2.6, 2.3, 2.5, 2.9, 2.2, 2.7]),
     ('g4_n3', [1.0, 2.0, 9.0]), ('g5_n2', [1.0, 2.0]), ('g6_n30', seq(30, 1.9, 10, 2) + [0.1]), ('g7_const', [3.0]*5),
     ('g8_n4', [1.0, 1.2, 1.1, 3.0]), ('g9_missing', [2.1, None, 2.2, 2.6, 2.3, 2.5, 9.8, 2.2]),
     ('g10_ties', [2.0, 2.0, 2.0, 2.0, 5.0, 2.0, 2.0]), ('g11_n10', [10.1, 10.3, 9.8, 10.0, 10.2, 9.9, 10.4, 10.1, 10.0, 12.9])]
for nm, v in G:
    for al in (0.05, 0.1, 0.01):
        add('gr', f'{nm}_a{al}', [v], alpha=al)
Q = [('q1', a), ('q3', [1.0, 2.0, 9.0]), ('q4', [1.0, 1.2, 1.1, 3.0]),
     ('q5', [0.189, 0.167, 0.187, 0.183, 0.186, 0.182, 0.181, 0.184, 0.177, 0.179]),
     ('q6_low', [0.5, 5.1, 5.2, 5.4, 5.3, 5.5, 5.15]), ('q7_n2', [1.0, 2.0]),
     ('q8_n11', [3.1, 3.2, 3.4, 3.3, 3.15, 3.25, 3.35, 3.3, 3.2, 3.28, 5.0]), ('q9_n20', seq(20, 1.9, 10, 2)),
     ('q10_n30', seq(30, 1.1, 10, 2)), ('q11_n31', seq(31, 1.4, 10, 2)), ('q12_const', [3.0]*5),
     ('q13_tie', [2.0, 2.0, 2.0, 5.0, 2.0]), ('q14_miss', [1.0, None, 1.2, 1.1, 3.0, 1.15]),
     ('q15_n8', [10.0, 10.1, 9.9, 10.2, 10.05, 9.95, 10.15, 11.0])]
for nm, v in Q:
    for al in (0.05, 0.1, 0.01):
        add('qt', f'{nm}_a{al}', [v], alpha=al)
# rowstats
add('row', 'r1', [a, b, c])
add('row', 'r2_missing', [[1.5, None, 3.5, None, 5.5, None], [1.0, 2.0, None, None, 4.0, None], [3.0, 9.0, 4.0, None, 7.0, None], [2.0, 2.5, 3.0, None, 9.0, None]])
add('row', 'r3_two', [a, b])
add('row', 'r4_one', [a])
json.dump(cases, open('cases2.json', 'w'))


def script(sel):
    out = ""
    for cs in cases:
        if cs['kind'] not in sel:
            continue
        out += cols(cs['cols'])
        nc = len(cs['cols']); rng = ",".join(str(i + 1) for i in range(nc))
        al = cs.get('alpha', 0.05); nm = cs['name']
        sh = 'R' + nm
        if cs['kind'] == 'sign':
            out += f"sign2 irng:=({rng}) null:={cs['null']} tail:={cs['tail']} alpha:={al} rt:=<new name:={sh}>;\n" + dump("%H", sh, 16, 5, nm)
        elif cs['kind'] == 'ks':
            out += f"kstest2 type:=1 irng:=({rng}) tail:={cs['tail']} alpha:={al} rt:=<new name:={sh}>;\n" + dump("%H", sh, 18, 5, nm)
        elif cs['kind'] == 'med':
            out += f"mediantest type:=1 irng:=({rng}) alpha:={al} rt:=<new name:={sh}>;\n" + dump("%H", sh, 18, 6, nm)
        elif cs['kind'] == 'fr':
            out += f"friedman type:=1 irng:=({rng}) alpha:={al} rt:=<new name:={sh}>;\n" + dump("%H", sh, 16, 7, nm)
        elif cs['kind'] == 'gr':
            out += 'double a, c, d, e; int b, f; string g$; a=0; b=0; c=0; d=0; e=0; f=0; g$="";\n'
            out += f"grubbs ix:=col(A) alpha:={al} ox:=a index:=b gstat:=c critical:=d pval:=e sig:=f conclusion:=g$ box:=0;\n#VARS {nm}\n?a\n?b\n?c\n?d\n?e\n?f\n$g$\n"
        elif cs['kind'] == 'qt':
            out += 'double a, c, d; int b, f; string g$; a=0; b=0; c=0; d=0; f=0; g$="";\n'
            out += f"qtest ix:=col(A) alpha:={al} ox:=a index:=b qstat:=c critical:=d sig:=f conclusion:=g$ box:=0;\n#VARS {nm}\n?a\n?b\n?c\n?d\n?f\n$g$\n"
        elif cs['kind'] == 'row':
            out += f"rowstats irng:=1:{nc} mean:=col({nc+1}) sd:=col({nc+2}) sem:=col({nc+3}) min:=col({nc+4}) max:=col({nc+5}) n:=col({nc+6}) sum:=col({nc+7});\n"
            out += dump("%H", None, nc + 7, len(cs['cols'][0]), nm)
    return out


if __name__ == '__main__':
    open(sys.argv[1], 'w').write(script(sys.argv[2].split(',')))

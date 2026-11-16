"""Own synthetic data for the three-way / repeated measures runs of T81."""
import math, itertools

def y3(i, a, b, c):
    return round(10 + 1.5*a + 0.8*b + 1.1*c + 0.9*((a*b) % 3) + 0.7*(a*c) - 0.5*b*c + 2*math.sin(7*i), 4)

def three(cells):
    rows = []; i = 0
    for (a, b, c), n in cells.items():
        for _ in range(n):
            i += 1; rows.append((a, b, c, y3(i, a, b, c)))
    order = sorted(range(len(rows)), key=lambda j: ((j*7) % len(rows))*1000 + j)
    return [rows[j] for j in order]

BAL = {(a, b, c): 2 for a in (1, 2) for b in (1, 2, 3) for c in (1, 2)}
UNB = dict(BAL); UNB.update({(1, 1, 1): 3, (1, 2, 2): 1, (2, 3, 1): 4, (2, 1, 2): 3, (1, 3, 2): 1})

def setup(rows, texty=False):
    A = ", ".join(str(r[0]) for r in rows); B = ", ".join(str(r[1]) for r in rows)
    C = ", ".join(str(r[2]) for r in rows); Y = ", ".join(repr(r[3]) for r in rows)
    return ("newbook;\ncol(A) = {%s};\nwks.addcol(); col(B) = {%s};\nwks.addcol(); col(C) = {%s};\nwks.addcol(); col(D) = {%s};\n" % (A, B, C, Y))

if __name__ == '__main__':
    import sys
    open(sys.argv[1], 'w').write(setup(three(BAL)))


def rm_data(nsub, levels, seed=0, effect=1.0, subjeff=3.0):
    """Subjects x levels table: y = 20 + subject effect + level effect + noise (own formula)."""
    rows = []
    for s in range(nsub):
        rows.append([round(20 + subjeff * math.sin(2.3 * s + 1) + effect * l * (1 + 0.2 * (s % 3)) + 1.5 * math.sin(5.1 * s * (l + 1) + seed), 4) for l in range(levels)])
    return rows


def rm_setup(table):
    nc = len(table[0]); s = "newbook;\n"
    for c in range(nc):
        vals = ", ".join(repr(row[c]) for row in table)
        s += ("wks.addcol(); " if c else "") + "col(%d) = {%s};\n" % (c + 1, vals)
    return s

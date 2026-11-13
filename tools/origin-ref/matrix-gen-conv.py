"""Writes matrix-conv.lines.txt: one Origin session that converts worksheets to matrices (w2m),
matrices to worksheets (m2w) and regular XYZ columns to a matrix (xyz_regular), and prints the
results as text lines (see matrix-gen-ops.py for the dump of a matrix).

Run:  python matrix-gen-conv.py > matrix-conv.lines.txt
"""

import sys

MDUMP = [
    'int nr = wks.nrows; int nc = wks.ncols;',
    'string d$ = ""; for(int r=1;r<=nr;r++){ for(int c=1;c<=nc;c++){ d$ = d$ + "$(mat(1)[r,c],*15) "; } d$ = d$ + "/ "; }',
    '$d$',
    'msetvalue formula:="x";',
    'string dx$ = "";for(int c=1;c<=nc;c++){dx$=dx$+"$(mat(1)[1,c],*15) ";}',
    '$dx$',
    'msetvalue formula:="y";',
    'string dy$ = "";for(int r=1;r<=nr;r++){dy$=dy$+"$(mat(1)[r,1],*15) ";}',
    '$dy$',
]

# Dump of the active worksheet: cell values by row, then the long name and units of each column.
WDUMP = [
    'int nr = wks.nrows; int nc = wks.ncols;',
    'string d$ = ""; for(int r=1;r<=nr;r++){ for(int c=1;c<=nc;c++){ d$ = d$ + "$(wcol(c)[r],*15) "; } d$ = d$ + "/ "; }',
    '$d$',
    'string ln$ = ""; for(int c=1;c<=nc;c++){ ln$ = ln$ + "[" + wcol(c)[L]$ + "] "; }',
    '$ln$',
]

out = []


def case(name):
    out.append('string cn$ = "CASE %s";' % name)
    out.append('$cn$')


def sheet(cols, longnames=None):
    """A new workbook whose columns hold the given lists (None = missing)."""
    out.append('newbook;')
    for k, vals in enumerate(cols):
        if k > 0:
            out.append('wks.addcol();')
        cells = ', '.join('-1.23456789E-300' if v is None else repr(v) for v in vals)
        out.append('col(%d) = {%s};' % (k + 1, cells))
    if longnames:
        for k, ln in enumerate(longnames):
            out.append('col(%d)[L]$ = "%s";' % (k + 1, ln))


def convert(name, w2m_args, cols, longnames=None):
    case(name)
    sheet(cols, longnames)
    out.append('w2m %s;' % w2m_args)
    out.append('string pn$ = page.name$;')
    out.append('$pn$')
    out.extend(MDUMP)


# --- w2m: no coordinates
data3 = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]]
convert('w2m-noxy', 'xy:=noxy', data3)
convert('w2m-default', '', data3)
convert('w2m-noxy-trim0-missing', 'xy:=noxy trim:=0',
        [[1, 2, None, 4], [5, 6, None, 8], [9, 10, None, 12]])
convert('w2m-noxy-trim1-missing', 'xy:=noxy trim:=1',
        [[1, 2, None, 4], [5, 6, None, 8], [9, 10, None, 12]])
convert('w2m-noxy-trim1-missingcol', 'xy:=noxy trim:=1',
        [[1, 2, 3, 4], [None, None, None, None], [9, 10, 11, 12]])
convert('w2m-noxy-ragged', 'xy:=noxy',
        [[1, 2, 3, 4], [5, 6, 7], [9, 10, 11, 12]])

# --- X across columns: first data row holds X, first column holds Y
conv_x = [[0, 100, 200, 300], [10, 1, 2, 3], [20, 4, 5, 6], [30, 7, 8, 9]]
convert('w2m-xcol-row1-ycol', 'xy:=xcol xlabel:=row1 ycol:=1', conv_x)
convert('w2m-xcol-row1-noycol', 'xy:=xcol xlabel:=row1 ycol:=0',
        [[0, 1, 2], [10, 11, 12], [20, 21, 22], [30, 31, 32]])
convert('w2m-xcol-param', 'xy:=xcol xlabel:=param param:=L ycol:=1',
        [[100, 200, 300], [1, 2, 3], [4, 5, 6], [7, 8, 9]],
        longnames=['', '10', '20', '30'])
convert('w2m-xcol-param-noycol', 'xy:=xcol xlabel:=param param:=L ycol:=0',
        [[1, 2, 3], [4, 5, 6], [7, 8, 9]], longnames=['5', '10', '15'])
# --- Y across columns
conv_y = [[0, 10, 20, 30], [100, 1, 2, 3], [200, 4, 5, 6], [300, 7, 8, 9]]
convert('w2m-ycol-row1-xcol', 'xy:=ycol ylabel:=row1 xcol:=1', conv_y)
convert('w2m-ycol-param', 'xy:=ycol ylabel:=param param:=L xcol:=1',
        [[10, 20, 30], [1, 2, 3], [4, 5, 6], [7, 8, 9]], longnames=['', '100', '200', '300'])
# --- uneven spacing of the coordinates
convert('w2m-xcol-uneven', 'xy:=xcol xlabel:=row1 ycol:=1',
        [[0, 100, 200, 300], [10, 1, 2, 3], [20, 4, 5, 6], [50, 7, 8, 9]])
convert('w2m-xcol-uneven-lr', 'xy:=xcol xlabel:=row1 ycol:=1 lr:=1',
        [[0, 100, 200, 300], [10, 1, 2, 3], [20, 4, 5, 6], [50, 7, 8, 9]])
convert('w2m-xcol-uneven-tol50', 'xy:=xcol xlabel:=row1 ycol:=1 tol:=50',
        [[0, 100, 200, 300], [10, 1, 2, 3], [20, 4, 5, 6], [50, 7, 8, 9]])
convert('w2m-xcol-descending', 'xy:=xcol xlabel:=row1 ycol:=1',
        [[0, 300, 200, 100], [30, 1, 2, 3], [20, 4, 5, 6], [10, 7, 8, 9]])
convert('w2m-xcol-tiny-uneven', 'xy:=xcol xlabel:=row1 ycol:=1',
        [[0, 100, 200, 300], [10, 1, 2, 3], [20, 4, 5, 6], [30.5, 7, 8, 9]])

# --- XYZ regular gridding
xs = [1, 2, 3]
ys = [10, 20, 30, 40]


def xyz(order):
    pts = [(x, y, x * 100 + y) for x in xs for y in ys]
    if order == 'y-major':
        pts = [(x, y, x * 100 + y) for y in ys for x in xs]
    elif order == 'shuffled':
        idx = [5, 0, 11, 3, 7, 2, 9, 4, 10, 1, 6, 8]
        pts = [pts[i] for i in idx]
    elif order == 'descending':
        pts = list(reversed(pts))
    return [[p[0] for p in pts], [p[1] for p in pts], [p[2] for p in pts]]


def grid(name, cols, args=''):
    case(name)
    sheet(cols)
    out.append('xyz_regular iz:=(1,2,3) %s;' % args)
    out.append('string pn$ = page.name$;')
    out.append('$pn$')
    out.extend(MDUMP)


grid('xyz-x-major', xyz('x-major'))
grid('xyz-y-major', xyz('y-major'))
grid('xyz-shuffled', xyz('shuffled'))
grid('xyz-descending', xyz('descending'))
cols = xyz('x-major')
# one point missing from the grid
grid('xyz-hole', [c[:5] + c[6:] for c in cols])
# uneven spacing
pts = [(x, y, x * 100 + y) for x in (0, 1, 3) for y in (0, 2)]
grid('xyz-uneven', [[p[0] for p in pts], [p[1] for p in pts], [p[2] for p in pts]])
# the same x, y repeated (duplicate point)
cols = xyz('x-major')
grid('xyz-duplicate', [c + [c[0]] for c in cols[:2]] + [cols[2] + [-5]])
# scattered points
pts = [(0.1, 0.2, 1), (0.9, 0.1, 2), (0.5, 0.5, 3), (0.2, 0.9, 4), (0.8, 0.8, 5), (0.4, 0.15, 6), (0.65, 0.35, 7)]
grid('xyz-scattered', [[p[0] for p in pts], [p[1] for p in pts], [p[2] for p in pts]])

# --- matrix to worksheet
def m2w(name, args, rows=3, cols=4, formula='i*10+j', x1=10, x2=40, y1=1, y2=3, missing=False):
    case(name)
    out.append('newbook mat:=1;')
    out.append('mdim cols:=%d rows:=%d x1:=%s x2:=%s y1:=%s y2:=%s;' % (cols, rows, x1, x2, y1, y2))
    out.append('msetvalue formula:="%s";' % formula)
    if missing:
        out.append('mat(1)[2,3] = -1.23456789E-300;')
    out.append('m2w %s;' % args)
    out.append('string pn$ = page.name$;')
    out.append('$pn$')
    out.extend(WDUMP)


m2w('m2w-direct-noxy', 'method:=direct xy:=noxy')
m2w('m2w-direct-default', '')
m2w('m2w-direct-xcol-row1-ycol', 'method:=direct xy:=xcol xlabel:=row1 ycol:=1')
m2w('m2w-direct-xcol-param', 'method:=direct xy:=xcol xlabel:=syspara sysparam:=p1 ycol:=1')
m2w('m2w-direct-xcol-none', 'method:=direct xy:=xcol xlabel:=none ycol:=1')
m2w('m2w-direct-ycol-row1-xcol', 'method:=direct xy:=ycol ylabel:=row1 xcol:=1')
m2w('m2w-xyz-xconst', 'method:=xyz option:=xconst')
m2w('m2w-xyz-yconst', 'method:=xyz option:=yconst')
m2w('m2w-xyz-default', 'method:=xyz')
m2w('m2w-xyz-missing-notrim', 'method:=xyz option:=xconst trim:=0', missing=True)
m2w('m2w-xyz-missing-trim', 'method:=xyz option:=xconst trim:=1', missing=True)

sys.stdout.write("\n".join(out) + "\n")

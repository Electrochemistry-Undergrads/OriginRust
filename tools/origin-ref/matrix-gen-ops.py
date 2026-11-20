"""Writes matrix-ops.lines.txt: one Origin session that runs the matrix operations on small matrices
of our own and prints each result (data, X coordinates, Y coordinates) as text lines.

Run:  python matrix-gen-ops.py > matrix-ops.lines.txt
Then: .\\run-labtalk-lines.ps1 -Script matrix-ops.lines.txt   (one Origin at a time; lock inside)

Every case prints a line  CASE <name>  (a LabTalk string), then the dump lines.
"""

import sys

# Dump the active matrix sheet: data row by row ('|' ends a row), then the X coordinate of every
# column and the Y coordinate of every row (read back through the set-values variables x and y;
# the data are dumped first, so overwriting them does no harm).
DUMP = [
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

out = []


def case(name, lines, dump=True):
    out.append('string cn$ = "CASE %s";' % name)
    out.append('$cn$')
    out.extend(lines)
    if dump:
        out.extend(DUMP)


def base(rows=3, cols=4, formula="i*10+j", x1=10, x2=40, y1=1, y2=3):
    return [
        'newbook mat:=1;',
        'mdim cols:=%d rows:=%d x1:=%s x2:=%s y1:=%s y2:=%s;' % (cols, rows, x1, x2, y1, y2),
        'msetvalue formula:="%s";' % formula,
    ]


# new matrix defaults
case('default-new', ['newbook mat:=1;'])
case('mdim-only-size', ['newbook mat:=1;', 'mdim cols:=5 rows:=2;'])
case('mdim-range', ['newbook mat:=1;', 'mdim cols:=3 rows:=3 x1:=-1 x2:=1 y1:=5 y2:=-5;'])
case('mdim-1col', ['newbook mat:=1;', 'mdim cols:=1 rows:=3 x1:=2 x2:=7 y1:=0 y2:=1;'])

# set values: variables
case('sv-ij', base(formula='i*100+j'))
case('sv-xy', base(formula='x*1000+y'))
case('sv-expr', base(formula='sin(x/10)*cos(y)+ln(i+j)'))
case('sv-z', base() + ['msetvalue formula:="mat(1)[i,j]*2+1";'])
case('sv-iif', base(formula='(i>1 && j<3) ? 100+i : -j'))
case('sv-missing-const', base(formula='(i==2 && j==3) ? -1.23456789E-300 : i*10+j'))
case('sv-missing-div', base(formula='(i==2 && j==3) ? 0/0 : i*10+j'))
case('sv-ncols', base(formula='i*10+j') + ['msetvalue formula:="x+y*0+wks.ncols";'])
case('sv-pow-int', base(formula='i^2 - j^0.5 + mod(i*j,3)'))

# structure operations
case('transpose', base() + ['mtranspose;'])
case('flip-h', base() + ['mflip flip:=horizontal;'])
case('flip-v', base() + ['mflip flip:=vertical;'])
case('flip-h-coord', base() + ['mflip flip:=horizontal coord:=1;'])
case('flip-v-coord', base() + ['mflip flip:=vertical coord:=1;'])
case('rot-cw90', base() + ['mrotate90 degree:=cw90;'])
case('rot-ccw90', base() + ['mrotate90 degree:=ccw90;'])
case('rot-180', base() + ['mrotate90 degree:=ccw180;'])
case('expand-default', base() + ['mexpand;'])
case('expand-3x2', base() + ['mexpand cols:=3 rows:=2;'])
case('expand-1x3', base() + ['mexpand cols:=1 rows:=3;'])
case('shrink-default', base(rows=4, cols=6, formula='i*10+j', x1=10, x2=60, y1=1, y2=4) + ['mshrink;'])
case('shrink-2x1', base(rows=4, cols=6, formula='i*10+j', x1=10, x2=60, y1=1, y2=4) + ['mshrink cols:=2 rows:=1;'])
case('shrink-uneven', base(rows=3, cols=5, formula='i*10+j', x1=10, x2=50, y1=1, y2=3) + ['mshrink cols:=2 rows:=2;'])
case('shrink-3x3', base(rows=6, cols=6, formula='i*i+j', x1=0, x2=5, y1=0, y2=5) + ['mshrink cols:=3 rows:=3;'])

# statistics
case('mstats', base(formula='i*10+j+x/7') + [
    'mat(1)[2,3] = -1.23456789E-300; mat(1)[3,1] = 1/0;',
    'mstats mean:=ms_mean sd:=ms_sd n:=ms_n min:=ms_min max:=ms_max sum:=ms_sum missing:=ms_miss;',
    'string st$ = "STATS $(ms_mean,*15) $(ms_sd,*15) $(ms_n,*15) $(ms_min,*15) $(ms_max,*15) $(ms_sum,*15) $(ms_miss,*15)";',
    '$st$',
], dump=False)

sys.stdout.write("\n".join(out) + "\n")

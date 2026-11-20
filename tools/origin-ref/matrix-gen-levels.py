"""Writes matrix-levels.lines.txt: Origin plots contour (colour fill), contour lines and heat maps of
ramps z = lo + (hi - lo) * u (u from 0 to 1 across the columns) so that the colour scale shows how
Origin rounds the range of Z and splits it into levels. Pictures go to OUT.

Run:  python matrix-gen-levels.py OUT > matrix-levels.lines.txt
"""

import sys

out_dir = sys.argv[1]  # an existing folder for the pictures, without a trailing backslash

if len(sys.argv) > 2 and sys.argv[2] == "mantissa":
    RANGES = [(0.1234567, 0.1234567 + m * 10.0) for m in (1.2, 1.8, 2.3, 2.8, 3.3, 3.8, 4.3, 4.8, 5.3, 5.8, 6.4, 7.0)]
    PREFIX = "m"
else:
    PREFIX = "lv"
    RANGES = [
        (0.1234567, 0.8765432),
        (13.37, 98.76),
        (-0.0456, 0.0789),
        (1234.5, 9876.5),
        (-73.1, 12.9),
        (0.00123, 0.00987),
        (5.55, 5.57),
        (100, 100.5),
        (-99123, 200456),
        (0, 1),
        (2, 11),
        (-3.3, 3.3),
    ]
lines = []
for k, (lo, hi) in enumerate(RANGES):
    for tag, template in [("c", "contour")] if PREFIX == "m" else [("c", "contour"), ("h", "HeatMap"), ("l", "contline")]:
        lines.append("newbook mat:=1;")
        lines.append("mdim cols:=9 rows:=9 x1:=0 x2:=8 y1:=0 y2:=8;")
        lines.append('msetvalue formula:="%r + (%r - (%r))*x/8";' % (lo, hi, lo))
        lines.append("worksheet -p 226 %s;" % template)
        lines.append(
            'expGraph type:=png path:="%s" filename:="%s_%s%02d" overwrite:=1 tr1.unit:=2 tr1.width:=800;' % (out_dir, PREFIX, tag, k)
        )
sys.stdout.write("\n".join(lines) + "\n")

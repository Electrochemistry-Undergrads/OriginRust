"""Writes matrix-pics.lines.txt: Origin makes contour plots and heat maps of small matrices of our own
and exports each as a PNG (800 pixels wide) into OUT. Pictures are kept in the git-ignored scratch folder.

Run:  python matrix-gen-pics.py OUT > matrix-pics.lines.txt
"""

import sys

out_dir = sys.argv[1]  # an existing folder for the pictures, without a trailing backslash

BUMP = "10*exp(-((x-1)^2+(y+0.5)^2)) - 5*exp(-((x+1)^2+(y-1)^2))"
lines = []


def pic(tag, formula, template, plot=226, cols=41, rows=41, x=(-3, 3), y=(-3, 3), extra=()):
    lines.append("newbook mat:=1;")
    lines.append("mdim cols:=%d rows:=%d x1:=%s x2:=%s y1:=%s y2:=%s;" % (cols, rows, x[0], x[1], y[0], y[1]))
    lines.append('msetvalue formula:="%s";' % formula)
    lines.append("worksheet -p %d %s;" % (plot, template))
    lines.extend(extra)
    lines.append('expGraph type:=png path:="%s" filename:="%s" overwrite:=1 tr1.unit:=2 tr1.width:=800;' % (out_dir, "o_" + tag))


for tag, template in [("contour", "contour"), ("contline", "contline"), ("contgray", "contgray"), ("heatmap", "HeatMap")]:
    pic(tag, BUMP, template)
pic("ramp01", "(x+3)/6*(y+3)/6", "contour")
pic("ramp100", "100*(x+3)/6*(y+3)/6", "contour")
pic("ramp-11", "sin(x)*cos(y)", "contour")
pic("ramp7", "7.3*(x+3)/6", "contour")
pic("narrow", "3.2+0.7*(x+3)/6*(y+3)/6", "contour")
pic("tiny", "x*0.00123*(y+3)", "contour")
pic("hm-ramp100", "100*(x+3)/6*(y+3)/6", "HeatMap")
pic("small-5x4", "i*10+j", "contour", cols=5, rows=4, x=(0, 4), y=(0, 3))
pic("small-5x4-hm", "i*10+j", "HeatMap", cols=5, rows=4, x=(0, 4), y=(0, 3))
sys.stdout.write("\n".join(lines) + "\n")

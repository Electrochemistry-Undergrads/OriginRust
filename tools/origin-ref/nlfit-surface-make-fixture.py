"""Turns the output of run-labtalk-lines.ps1 on nlfit-surface-suite.lines.txt into
crates/fit/tests/data/origin_surface.json.
usage: nlfit-surface-make-fixture.py suite.out nlfit-surface-meta.json out.json"""
import json, re, sys

lines = open(sys.argv[1], encoding="utf8").read().split("\n")
meta = json.load(open(sys.argv[2]))

sheets, i = {}, 0
while i < len(lines):
    m = re.match(r"#SHEET (\S+) (\d+) (\d+)", lines[i])
    if m:
        n = int(m.group(2))
        sheets.setdefault(m.group(1), []).append([l.split("\t") for l in lines[i + 1:i + 1 + n]])
        i += n
    i += 1

blocks, cur = [], {}
for l in lines:
    m = re.match(r"([iv])_(\w+) = (\S+)$", l)
    if m:
        cur.setdefault(m.group(1), {})[m.group(2)] = float(m.group(3))
    m = re.match(r"nit = (\S+)$", l)
    if m:
        cur["nit"] = float(m.group(1))
    if l.startswith("#SHEET") and "FitSurface1" in l:
        blocks.append(cur)
        cur = {}

def f(s):
    return float(s) if s != "" else None

out = []
for k, mt in enumerate(meta, 1):
    data = sheets["[Book%d]Sheet1" % k][0]
    rep = sheets["[Book%d]FitSurface1" % k][0]
    np_ = len(mt["params"])
    p0, sc = 7, 12
    params = [{"name": mt["params"][r], "value": f(rep[r][p0]), "se": f(rep[r][p0 + 1]), "t": f(rep[r][p0 + 2]),
               "prob": f(rep[r][p0 + 3]), "dep": f(rep[r][p0 + 4])} for r in range(np_)]
    stats = {"n": int(float(rep[0][sc])), "dof": int(float(rep[1][sc])), "red_chi": float(rep[2][sc]),
             "rss": float(rep[3][sc]), "r2": float(rep[4][sc]), "adj_r2": float(rep[5][sc]), "status": rep[6][sc]}
    ac = next(j for j in range(sc + 1, len(rep[0]))
              if rep[1][j] != "" and rep[2][j] != "" and rep[1][j].replace(".", "").isdigit()
              and int(float(rep[1][j])) == stats["dof"] and int(float(rep[2][j])) == stats["n"])
    anova = {"model_df": int(float(rep[0][ac])), "model_ss": float(rep[0][ac + 1]), "model_ms": float(rep[0][ac + 2]),
             "error_df": int(float(rep[1][ac])), "error_ss": float(rep[1][ac + 1]),
             "utotal_df": int(float(rep[2][ac])), "utotal_ss": float(rep[2][ac + 1]),
             "total_df": int(float(rep[3][ac])), "total_ss": float(rep[3][ac + 1]),
             "f": float(rep[0][ac + 3]), "prob_f": float(rep[0][ac + 4])}
    b = blocks[k - 1]
    case = {"name": "%s%s" % (mt["func"].lower(), (" " + mt["tag"]) if mt["tag"] else ""), "func": mt["func"],
            "x": [f(r[0]) for r in data], "y": [f(r[1]) for r in data], "z": [f(r[2]) for r in data],
            "origin_initial": b["i"], "origin_iterations": int(b["nit"]), "params_origin": params,
            "stats": stats, "anova": anova}
    out.append(case)
json.dump(out, open(sys.argv[3], "w"), indent=1)
print(len(out), "cases")

import json, re, sys

src = sys.argv[1]
cases_meta = json.load(open(sys.argv[2]))
lines = open(src, encoding="utf8").read().split("\n")

# split into blocks per "#SHEET ... Sheet1"
def sheets(lines):
    res = {}
    i = 0
    while i < len(lines):
        m = re.match(r"#SHEET (\S+) (\d+) (\d+)", lines[i])
        if m:
            n = int(m.group(2))
            res.setdefault(m.group(1), []).append([l.split("\t") for l in lines[i + 1:i + 1 + n]])
            i += n
        i += 1
    return res

sh = sheets(lines)

def vars_before(k):
    # collect i_/v_/nit variables per case by scanning text between SHEET markers
    pass

# variable lines in order: each case has i_*, v_*, nit before its Sheet1 marker
blocks = []
cur = {}
for l in lines:
    m = re.match(r"([iv])_(\w+) = (\S+)$", l)
    if m:
        cur.setdefault(m.group(1), {})[m.group(2)] = float(m.group(3))
    m = re.match(r"nit = (\S+)$", l)
    if m:
        cur["nit"] = float(m.group(1))
    if l.startswith("#SHEET") and "FitNL1" in l:
        blocks.append(cur)
        cur = {}

out = []
for k, meta in enumerate(cases_meta, 1):
    data = sh["[Book%d]Sheet1" % k][0]
    rep = sh["[Book%d]FitNL1" % k][0]
    x = [float(r[0]) for r in data]
    y = [float(r[1]) for r in data]
    err = [float(r[2]) for r in data]
    np_ = len(meta["params"])
    weighted = rep[1][7].startswith("[")
    p0 = 8 if weighted else 7
    params = []
    for i in range(np_):
        r = rep[i]
        params.append({"name": meta["params"][i], "value": float(r[p0]), "se": float(r[p0 + 1]) if r[p0 + 1] else None,
                       "t": float(r[p0 + 2]) if r[p0 + 2] else None, "prob": float(r[p0 + 3]) if r[p0 + 3] else None,
                       "dep": float(r[p0 + 4]) if r[p0 + 4] else None})
    sc = p0 + 5
    stats = {"n": int(float(rep[0][sc])), "dof": int(float(rep[1][sc])), "red_chi": float(rep[2][sc]),
             "rss": float(rep[3][sc]), "r2": float(rep[4][sc]), "adj_r2": float(rep[5][sc]), "status": rep[6][sc]}
    # anova column: first column after sc where row1 == dof and row2 == n
    ac = None
    for j in range(sc + 1, len(rep[0])):
        try:
            if int(float(rep[1][j])) == stats["dof"] and int(float(rep[2][j])) == stats["n"]:
                ac = j
                break
        except ValueError:
            pass
    anova = {"model_df": int(float(rep[0][ac])), "model_ss": float(rep[0][ac + 1]), "model_ms": float(rep[0][ac + 2]),
             "error_df": int(float(rep[1][ac])), "error_ss": float(rep[1][ac + 1]), "error_ms": float(rep[1][ac + 2]),
             "utotal_df": int(float(rep[2][ac])), "utotal_ss": float(rep[2][ac + 1]),
             "total_df": int(float(rep[3][ac])), "total_ss": float(rep[3][ac + 1]),
             "f": float(rep[0][ac + 3]), "prob_f": float(rep[0][ac + 4])}
    b = blocks[k - 1]
    case = dict(meta)
    case.update({"x": x, "y": y, "err": err if meta.get("weight") == "ins" else None,
                 "origin_initial": b["i"], "origin_iterations": int(b["nit"]),
                 "params_origin": params, "stats": stats, "anova": anova})
    out.append(case)

json.dump(out, open(sys.argv[3], "w"), indent=1)
print(len(out), "cases")

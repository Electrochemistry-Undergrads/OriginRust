"""Global-fit reference runs for NLFit in Origin (mode:=global), and the fixture maker.

  python nlfit-global-gen.py gen   nlfit-global.lines.txt nlfit-global-meta.json
  run-labtalk-lines.ps1 -Script nlfit-global.lines.txt > nlfit-global.out
  python nlfit-global-gen.py make  nlfit-global.out nlfit-global-meta.json origin_global.json

Data: column A is x, B..D hold the datasets, E..G their errors, H the row index used by the noise
terms. A shared parameter is set with tt.s_<name> = 1; a fixed one with tt.f_<name> = 1 (the name
of dataset k >= 2 ends in _k).
"""
import json
import re
import sys

N = 20
NOISE = ["0.05*sin(37*col(H)) + 0.04*sin(101.3*col(H))", "0.04*sin(41*col(H)) + 0.03*sin(97.3*col(H))",
         "0.03*sin(29*col(H)) + 0.03*sin(83.1*col(H))"]
GAUSS = [("1", "3", "5", "1.5"), ("0.4", "2", "5.2", "1.7")]
CASES = [
    dict(name="gauss_share_xc", func="Gauss", params=["y0", "xc", "w", "A"], nsets=2, share=["xc"], kind="gauss"),
    dict(name="gauss_no_share", func="Gauss", params=["y0", "xc", "w", "A"], nsets=2, share=[], kind="gauss"),
    dict(name="gauss_share_xc_w", func="Gauss", params=["y0", "xc", "w", "A"], nsets=2, share=["xc", "w"],
         kind="gauss"),
    dict(name="expdec1_share_t1", func="ExpDec1", params=["y0", "A1", "t1"], nsets=3, share=["t1"], kind="exp"),
    dict(name="gauss_share_xc_instr", func="Gauss", params=["y0", "xc", "w", "A"], nsets=2, share=["xc"],
         kind="gauss", weight="ins"),
    dict(name="gauss_share_xc_fixed_y0", func="Gauss", params=["y0", "xc", "w", "A"], nsets=2, share=["xc"],
         kind="gauss", fixed={"y0": 0.9, "y0_2": 0.4}),
]


def comp_names(c):
    out = []
    for p in c["params"]:
        if p in c["share"]:
            out.append(p)
        else:
            out += [p if d == 0 else "%s_%d" % (p, d + 1) for d in range(c["nsets"])]
    return out


def ycol(c, d):
    n = NOISE[d]
    if c["kind"] == "gauss":
        y0, a, xc, w = GAUSS[d]
        return "%s + %s*exp(-2*((col(A)-%s)/%s)^2) + %s" % (y0, a, xc, w, n)
    # exponential decays with a common time constant of 2
    return "%s + %s*exp(-col(A)/2) + %s" % (["0.3", "0.8", "1.2"][d], ["2.5", "1.5", "3.5"][d], n)


def gen(out, meta):
    L = []
    for k, c in enumerate(CASES, 1):
        cols = "BCD"[: c["nsets"]]
        ecols = "EFG"[: c["nsets"]]
        L += ["newbook;", "col(A) = data(0.5, 10, 0.5);"]
        for ch in "BCDEFG":
            L.append("col(%s) = col(A)*0;" % ch)
        L.append("col(H) = data(1, %d, 1);" % N)
        for d in range(c["nsets"]):
            L.append("col(%s) = %s;" % (cols[d], ycol(c, d)))
            if c.get("weight"):
                L.append("col(%s) = 0.0%d + 0.01*(1+sin(%d*col(H)));" % (ecols[d], 2 + d, 5 + 2 * d))
        sets = []
        for d in range(c["nsets"]):
            if c.get("weight"):
                sets.append("(1,%d,%d)" % (2 + d, 5 + d))
            else:
                sets.append("(1,%d)" % (2 + d))
        wt = " weight:=ins" if c.get("weight") else ""
        L.append("nlbegin iy:=(%s) func:=%s mode:=global nltree:=tt%s;" % (",".join(sets), c["func"], wt))
        for p in c["share"]:
            L.append("tt.s_%s = 1;" % p)
        for key, v in c.get("fixed", {}).items():
            L.append("tt.%s = %r; tt.f_%s = 1;" % (key, v, key))
        for n in comp_names(c):
            L += ["double i_%s = tt.%s;" % (n, n), "?i_%s" % n]
        L += ["nlfit;", "nlend output:=1;", "#[Book%d]Sheet1" % k, "#[Book%d]FitNL1" % k, "$page.name$"]
    open(out, "w").write("\n".join(L) + "\n")
    json.dump(CASES, open(meta, "w"), indent=1)


def make(src, meta, out):
    lines = open(src, encoding="utf8").read().split("\n")
    cases = json.load(open(meta))
    sh = {}
    i = 0
    while i < len(lines):
        m = re.match(r"#SHEET (\S+) (\d+) (\d+)", lines[i])
        if m:
            n = int(m.group(2))
            sh[m.group(1)] = [ln.split("\t") for ln in lines[i + 1:i + 1 + n]]
            i += n
        i += 1
    inits, cur = [], {}
    for ln in lines:
        m = re.match(r"i_(\w+) = (\S+)$", ln)
        if m:
            cur[m.group(1)] = float(m.group(2))
        if ln.startswith("#SHEET") and "FitNL1" in ln:
            inits.append(cur)
            cur = {}
    res = []
    for k, c in enumerate(cases, 1):
        nd = c["nsets"]
        data = sh["[Book%d]Sheet1" % k]
        rep = sh["[Book%d]FitNL1" % k]
        x = [float(r[0]) for r in data]
        ys = [[float(r[1 + d]) for r in data] for d in range(nd)]
        errs = [[float(r[4 + d]) for r in data] for d in range(nd)] if c.get("weight") else None
        weighted = rep[1][7].startswith("[")
        p0 = 8 if weighted else 7
        block = len(rep) // nd
        shared = [p in c["share"] for p in c["params"]]
        params = []
        for d in range(nd):
            for j, p in enumerate(c["params"]):
                name = p if d == 0 else "%s_%d" % (p, d + 1)
                if shared[j] and d > 0:
                    continue
                r = rep[d * block + j]

                def g(i, r=r):
                    return float(r[i]) if r[i] != "" else None

                params.append({"name": name, "value": g(p0), "se": g(p0 + 1), "t": g(p0 + 2), "prob": g(p0 + 3),
                               "dep": g(p0 + 4), "dataset": d, "param": p})
        sc = p0 + 5
        gcol = sc + nd  # the column for all datasets together
        stats = {"n": int(float(rep[0][gcol])), "dof": int(float(rep[1][gcol])), "red_chi": float(rep[2][gcol]),
                 "rss": float(rep[3][gcol]), "r2": float(rep[4][gcol]), "adj_r2": float(rep[5][gcol]),
                 "status": rep[6][gcol], "ds_rss": [float(rep[3][sc + d]) for d in range(nd)],
                 "ds_r2": [float(rep[4][sc + d]) for d in range(nd)]}
        ac = None
        for j in range(gcol + 1, len(rep[0])):
            try:
                if int(float(rep[1][j])) == stats["dof"] and int(float(rep[2][j])) == stats["n"]:
                    ac = j
                    break
            except ValueError:
                pass
        anova = {"model_df": int(float(rep[0][ac])), "model_ss": float(rep[0][ac + 1]),
                 "error_df": int(float(rep[1][ac])), "error_ss": float(rep[1][ac + 1]),
                 "utotal_ss": float(rep[2][ac + 1]), "total_df": int(float(rep[3][ac])),
                 "total_ss": float(rep[3][ac + 1]), "f": float(rep[0][ac + 3])}
        case = dict(c)
        case.update({"x": x, "ys": ys, "errs": errs, "origin_initial": inits[k - 1], "params_origin": params,
                     "stats": stats, "anova": anova})
        res.append(case)
    json.dump(res, open(out, "w"), indent=1)


if sys.argv[1] == "gen":
    gen(sys.argv[2], sys.argv[3])
else:
    make(sys.argv[2], sys.argv[3], sys.argv[4])

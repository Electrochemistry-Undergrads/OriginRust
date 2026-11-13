"""Turns the raw output of the fitcmpmodel reference runs into crates/fit/tests/data/origin_fitcmp.json.

  run-labtalk-safe.ps1 -Script fitcmp-suite1.lines.txt > suite1.out      (same for suite2, suite3)
  python fitcmp-make-fixture.py origin_fitcmp.json fitcmp-suite1.lines.txt suite1.out ...

The scripts fit two functions to one dataset (nlbegin, nlfit, nlend output:=1), run `fitcmpmodel` with
`rt:=<new name:=cmpA>` and dump the report sheet. Its layout (columns from 0): 3 and 4 the model rows
(name, n, parameters), 5 the conclusions (one row per comparison that was switched on, in the order
AIC, BIC, F-test), then the AIC table (RSS, n, parameters, AIC, weight), the BIC table (RSS, n,
parameters, BIC, BIC minus the smaller BIC) and the F-test (F, df1, df2, Prob>F); a switched-off
comparison leaves no columns.
"""
import json
import re
import sys


def cells(lines, k):
    for i, l in enumerate(lines):
        if l.startswith("#SHEET [Book%d]cmpA " % k):
            n = int(l.split()[2])
            return [r.split("\t") for r in lines[i + 1:i + 1 + n]]
    return None


def num(rows, r, c):
    try:
        return float(rows[r][c]) if rows[r][c] != "" else None
    except IndexError:
        return None


def main():
    out, pairs = sys.argv[1], sys.argv[2:]
    cases = []
    for src, raw in zip(pairs[0::2], pairs[1::2]):
        script = open(src, encoding="utf8").read().split("\n")
        lines = open(raw, encoding="utf8").read().split("\n")
        cmds = [l for l in script if l.startswith("fitcmpmodel")]
        for k, cmd in enumerate(cmds, 1):
            rows = cells(lines, k)
            opt = lambda key, d: re.search(key + r":=(\S+?)[ ;]", cmd + " ").group(1) if re.search(key + r":=", cmd) else d
            use = {"aic": opt("aic", "1") == "1", "bic": opt("bic", "0") == "1", "ftest": opt("ftest", "0") == "1"}
            names = [opt("name1", "Model1"), opt("name2", "Model2")]
            models = []
            for c in (3, 4):
                models.append({"function": rows[3][c], "n": int(float(rows[4][c])), "k": int(float(rows[5][c]))})
            col, picks = 6, []
            case = {"source": "%s case %d" % (src, k), "use": use, "names": names, "models": models}
            conc = [rows[r][5] for r in range(3) if rows[r][5] != ""]
            if use["aic"]:
                case["aic"] = {"rss": [num(rows, 0, col), num(rows, 1, col)], "aic": [num(rows, 0, col + 3), num(rows, 1, col + 3)],
                               "weight": [num(rows, 0, col + 4), num(rows, 1, col + 4)]}
                col += 5
            if use["bic"]:
                case["bic"] = {"bic": [num(rows, 0, col + 3), num(rows, 1, col + 3)], "delta": [num(rows, 0, col + 4), num(rows, 1, col + 4)]}
                col += 5
            if use["ftest"]:
                case["f"] = {"f": num(rows, 0, col), "df1": num(rows, 0, col + 1), "df2": num(rows, 0, col + 2), "p": num(rows, 0, col + 3)}
            for m, c in zip(models, (0, 1)):
                pass
            # the residual sums of squares come from the first table switched on
            first = col0 = 6
            case["rss"] = [num(rows, 0, first), num(rows, 1, first)]
            case["conclusions"] = conc
            cases.append(case)
    json.dump(cases, open(out, "w"), indent=1)
    print(len(cases), "cases")


main()

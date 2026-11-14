"""Writes the LabTalk script (lines format) of the power / sample size reference runs: Origin's library functions
for the one-sample, two-sample and paired t tests and the one-way ANOVA, with inputs generated here.
usage: python g2.py cases2.json script.lines.txt"""
import json, random, sys

random.seed(20261114)
cases = []

def add(name, kind, mode, tail, alpha, a, b, sd, vals):
    cases.append(dict(name=name, kind=kind, mode=mode, tail=tail, alpha=alpha, a=a, b=b, sd=sd, vals=vals))

KINDS = {1: 't1', 2: 't2', 3: 'tp', 4: 'an'}
sizes_small = [2, 3, 4, 5, 6, 8, 10, 15, 20, 30, 50, 100]
for kind in (1, 2, 3):
    for tail in (0, 1):
        for rep, (alpha, a, b, sd) in enumerate([(0.05, 5, 6, 1.5), (0.01, 10, 10.8, 2.0), (0.10, 3, 2.2, 0.9), (0.05, 1.0, 1.2, 0.2)]):
            tag = f'{KINDS[kind]}_t{tail}_r{rep}'
            add(tag + '_pow', kind, 0, tail, alpha, a, b, sd, sizes_small)
            add(tag + '_n', kind, 1, tail, alpha, a, b, sd, [0.2, 0.5, 0.7, 0.8, 0.9, 0.95, 0.99])
for rep, (alpha, lv, css, sd) in enumerate([(0.05, 3, 2.0, 1.0), (0.01, 4, 1.5, 2.0), (0.10, 2, 0.5, 1.0), (0.05, 5, 0.4, 1.2), (0.05, 3, 0.1, 1.0)]):
    add(f'an_r{rep}_pow', 4, 0, 0, alpha, lv, css, sd, sizes_small + [2.5, 7.3])
    add(f'an_r{rep}_n', 4, 1, 0, alpha, lv, css, sd, [0.2, 0.5, 0.7, 0.8, 0.9, 0.95, 0.99])
# edge cases
add('t2_zero_effect_pow', 2, 0, 0, 0.05, 5, 5, 1.5, [5, 10])
add('t2_zero_effect_n', 2, 1, 0, 0.05, 5, 5, 1.5, [0.5, 0.8])
add('t2_tiny_effect_n', 2, 1, 0, 0.05, 5, 5.01, 1.5, [0.5, 0.8])
add('t2_big_effect_n', 2, 1, 0, 0.05, 5, 50, 1.0, [0.5, 0.8, 0.99])
add('t2_below_lowest', 2, 1, 0, 0.05, 5, 6, 1.5, [0.05, 0.07])
add('t2_mixed_lowest', 2, 1, 0, 0.05, 5, 6, 1.5, [0.05, 0.8, 0.07, 0.9])
add('t2_power_one', 2, 1, 0, 0.05, 5, 6, 1.5, [0.8, 1.0])
add('t2_power_zero', 2, 1, 0, 0.05, 5, 6, 1.5, [0.0, 0.8])
add('t2_frac_ok', 2, 0, 0, 0.05, 5, 6, 1.5, [10, 12.0])
add('t2_frac', 2, 0, 0, 0.05, 5, 6, 1.5, [10, 12.5])
add('t1_frac', 1, 0, 0, 0.05, 5, 6, 1.5, [10, 12.5])
add('t2_n_two', 2, 0, 0, 0.05, 5, 6, 1.5, [2, 3])
add('t1_n_two', 1, 0, 0, 0.05, 5, 6, 1.5, [2, 3])
add('t2_n_one', 2, 0, 0, 0.05, 5, 6, 1.5, [1, 10])
add('t2_n_below', 2, 0, 0, 0.05, 5, 6, 1.5, [1.5, 10])
add('t2_sd_zero', 2, 0, 0, 0.05, 5, 6, 0, [5, 10])
add('t2_alpha_one', 2, 0, 0, 1.0, 5, 6, 1.5, [5, 10])
add('t2_alpha_big', 2, 0, 0, 0.5, 5, 6, 1.5, [5, 10])
add('t2_neg_effect_pow', 2, 0, 1, 0.05, 6, 5, 1.5, [5, 10, 30])
add('t2_neg_effect_n', 2, 1, 1, 0.05, 6, 5, 1.5, [0.5, 0.8])
add('an_one_level', 4, 1, 0, 0.05, 1, 2, 1, [0.5, 0.8])
add('an_zero_css', 4, 1, 0, 0.05, 3, 0, 1, [0.5, 0.8])
add('an_n_one_pow', 4, 0, 0, 0.05, 3, 2, 1, [1, 2, 3])
add('an_levels_frac', 4, 0, 0, 0.05, 3.5, 2, 1, [5, 10])
add('an_big_n', 4, 1, 0, 0.05, 3, 0.02, 1, [0.5, 0.8])
add('an_below', 4, 1, 0, 0.05, 3, 20, 1, [0.05, 0.5, 0.99])
add('an_below_mild', 4, 1, 0, 0.05, 3, 2, 1, [0.05, 0.12, 0.5])
add('t1_big_effect_n', 1, 1, 0, 0.05, 5, 50, 1.0, [0.5, 0.8, 0.99])
add('t2_big_effect2_n', 2, 1, 0, 0.05, 5, 15, 1.0, [0.5, 0.8, 0.99])
add('t2_sd_neg', 2, 0, 0, 0.05, 5, 6, -1.5, [5, 10])
add('t2_alpha_tiny', 2, 0, 0, 1e-8, 5, 6, 1.5, [5, 10])
add('an_alpha_tiny', 4, 0, 0, 1e-8, 3, 2, 1, [5, 10])
add('t2_large_n', 2, 0, 0, 0.05, 5, 5.5, 1.5, [1000, 5000, 100000])
json.dump(cases, open(sys.argv[1], 'w'))

out = "newbook;\nint rc = run.LoadOC(%WORK%stats4/stats4-pss.c, 16);\n"
for ci, c in enumerate(cases):
    if ci:
        out += "newbook;\n"
    vals = ', '.join(repr(float(v)) for v in c['vals'])
    out += f"col(1) = {{{vals}}};\n"
    out += f"double r{ci} = sheaf_pss({c['kind']}, {c['mode']}, {c['tail']}, {c['alpha']}, {c['a']}, {c['b']}, {c['sd']}, 0);\n?r{ci}\n#[Book{ci+1}]Sheet1\n"
open(sys.argv[2], 'w').write(out)

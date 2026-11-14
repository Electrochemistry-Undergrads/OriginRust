"""Turns the raw output of the discfreqs runs (g1.py) into crates/calc/tests/fixtures/discfreq_origin.rs and a compact
copy of Origin's answers (discfreq-origin.compact.json). usage: python make-fixture1.py RAW.out.txt fixture.rs compact.json"""
import json, sys
from parse1 import parse

raw, fixture, compact = sys.argv[1:4]
cases = {c['name']: c for c in json.load(open('cases1.json'))}
kept, out = {}, []
for name, r in parse(raw).items():
    c = cases[name]
    if c['kind'] == 'idx':
        continue  # the indexed form is not built
    nc = r['ncols']
    names = [r['cols'][k]['lname'] for k in range(1, nc + 1)]
    units = [r['cols'][k]['unit'] for k in range(1, nc + 1)]
    rows = []
    i = 1
    while (1, i) in r['cells'] or (2, i) in r['cells']:
        rows.append([r['cells'].get((k, i), '') for k in range(1, nc + 1)])
        i += 1
    kept[name] = dict(opts=c['opts'], names=names, units=units, rows=rows)
    d = dict(x.split(':=') for x in c['opts'].split())
    def data():
        if c['kind'] == 'text':
            return ', '.join(f'T({json.dumps(v)})' for v in c['data'])
        return ', '.join('M' if v is None else f'N({float(v)!r})' for v in c['data'])
    num = lambda s: s if ('.' in s or 'e' in s) else s + '.0'
    out.append(f'''    DiscCase {{
        name: {json.dumps(name)},
        data: &[{data()}],
        sort: {json.dumps(d.get('sort', 'cdesc'))},
        case_sensitive: {str(d.get('sensitive') == '1').lower()},
        percent: {str(d.get('show') == 'percent').lower()},
        names: &[{', '.join(json.dumps(n) for n in names)}],
        units: &[{', '.join(json.dumps(u) for u in units)}],
        rows: &[{', '.join('&[' + ', '.join(json.dumps(v) for v in row) + ']' for row in rows)}],
    }},
''')
head = '''//! Origin's answers to the discfreqs runs of tools/origin-ref/stats5/g1.py (made by make-fixture1.py; the compact
//! output is tools/origin-ref/stats5/discfreq-origin.compact.json). Do not edit by hand. `rows` are the cells as
//! Origin shows them (five decimals); `names` and `units` are the long names and units of the result columns.
#![allow(dead_code)]

use sheaf_calc::Category;

pub struct DiscCase {
    pub name: &'static str,
    pub data: &'static [Cat],
    pub sort: &'static str,
    pub case_sensitive: bool,
    pub percent: bool,
    pub names: &'static [&'static str],
    pub units: &'static [&'static str],
    pub rows: &'static [&'static [&'static str]],
}

/// A cell of the input: text, number or missing.
pub enum Cat {
    T(&'static str),
    N(f64),
    M,
}
use Cat::{M, N, T};

impl Cat {
    pub fn category(&self) -> Category {
        match self {
            T(s) => Category::Text(s.to_string()),
            N(x) => Category::Number(*x),
            M => Category::Missing,
        }
    }
}

pub const DISC_CASES: &[DiscCase] = &[
'''
open(fixture, 'w', newline='\n').write(head + ''.join(out) + '];\n')
json.dump(kept, open(compact, 'w'), indent=0)
print(len(out), 'cases')

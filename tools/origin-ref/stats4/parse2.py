"""Parses the raw output of a run of g2.py / gN.py scripts: {case_index: (return_value, rows)}; each row is the list of
cell strings of a dumped worksheet row. A case whose sheet could not be read has no rows."""
import re, sys

def parse(path):
    res = {}; cur = None
    for line in open(path, encoding='utf-8', errors='replace').read().split('\n'):
        line = line.rstrip('\r')
        m = re.match(r'r(\d+) = (\S+)$', line)
        if m:
            cur = int(m.group(1)); res[cur] = [float(m.group(2)), []]; continue
        if line.startswith('#') or line.startswith('[') or line.startswith('<'):
            if cur is not None and (line.startswith('#NOARRAY') or line.startswith('[')):
                pass
            if line.startswith('#SHEET') or line.startswith('#NOARRAY'):
                continue
            cur = None if line.startswith('[') else cur
            continue
        if cur is not None and '\t' in line + '\t':
            res[cur][1].append(line.split('\t'))
    return res

if __name__ == '__main__':
    for k, (rc, rows) in sorted(parse(sys.argv[1]).items()):
        print(k, rc, rows[:3])

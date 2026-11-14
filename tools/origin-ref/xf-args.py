"""Lists the arguments of an Origin X-Function file: name, label, type, default and choices.

Only the interface metadata (what Origin's help and dialog show) is printed; the file's code is not
read or printed, under the clean-room rule. usage: python xf-args.py FILE.OXF [...]"""
import re
import sys

FIELD = r'<{tag} [^>]*>(.*?)</{tag}>'


def field(block, tag):
    m = re.search(FIELD.format(tag=tag), block, re.S)
    return m.group(1).strip() if m else ''


for path in sys.argv[1:]:
    text = open(path, 'rb').read().decode('latin-1')
    print(f'== {path}')
    for m in re.finditer(r'<v\d+ _XFUNCTION_VAR="1"[^>]*>(.*?)</v\d+>', text, re.S):
        block = re.sub(r'<help[jgc] .*?</help[jgc]>', '', m.group(1), flags=re.S)
        default = field(block, 'valdata')
        default = re.sub(r'<[^>]+Data="([^"]*)"[^>]*>.*', r'\1', default, flags=re.S)
        default = default.replace('&lt;', '<').replace('&gt;', '>')
        print(f"{field(block, 'name')}\t{field(block, 'description')}\t{field(block, 'type')}\t"
              f"default={default[:60]}\tchoices={field(block, 'comboString')[:200]}")

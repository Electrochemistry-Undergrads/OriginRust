#!/usr/bin/env python3
"""Lists the curve records and layer headers of the windows of an .opj file and prints, byte by byte,
how each differs from the same record of a reference window (our own tool, builds on opj-dump-blocks.py).

Usage: opj-diff-curves.py FILE.opj REFERENCE_WINDOW [OTHER_FILE.opj]
  Every window of FILE (or of OTHER_FILE when given; the workbook windows have no layers of curves and are skipped) is compared with the first curve record and the
  layer header of REFERENCE_WINDOW in FILE.
"""
import importlib.util
import os
import sys

here = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("dump", os.path.join(here, "opj-dump-blocks.py"))
dump = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dump)


def windows(path):
    """{window name: [(layer header, [curve records], [window header])]}"""
    r = dump.Reader(open(path, "rb").read())
    r.line(); r.block(); r.block()
    while r.block():
        r.block(); r.block()
    out = {}
    while True:
        wh = r.block()
        if not wh:
            break
        name = wh[2:27].split(b"\0")[0].decode("latin1")
        layers = []
        while True:
            lh = r.block()
            if not lh:
                break
            while True:
                ah = r.block()
                if not ah:
                    break
                b1 = r.block(); b2 = r.block()
                import struct
                if len(b1) in (0x5E, 0x0A) and len(b2) == 4:
                    n = struct.unpack("<I", b2)[0]
                    mh = r.raw(n)
                    while True:
                        c1 = r.block(); c2 = r.block()
                        if len(c1) in (0x5E, 0x0A) and len(c2) == 4:
                            mh = r.raw(struct.unpack("<I", c2)[0]); continue
                        r.block(); break
                else:
                    r.block()
            curves = []
            notes = []
            while True:
                rec = r.block()
                if not rec:
                    break
                notes.append(r.block())
                curves.append(rec)
            r.block()
            for g in range(3):
                while r.block():
                    pass
            layers.append((lh, curves, wh, notes))
        out[name] = layers
    return out


def diff(a, b, limit=60):
    n = max(len(a), len(b))
    a = a.ljust(n, b"\0"); b = b.ljust(n, b"\0")
    d = [f"{i:#x}:{a[i]:02x}>{b[i]:02x}" for i in range(n) if a[i] != b[i]]
    return " ".join(d[:limit]) + (f" ... ({len(d)} bytes differ)" if len(d) > limit else "")


def main():
    path, ref = sys.argv[1], sys.argv[2]
    other = sys.argv[3] if len(sys.argv) > 3 else path
    base = windows(path)[ref][0]
    for name, layers in windows(other).items():
        for li, (lh, curves, wh, notes) in enumerate(layers):
            if name == ref and other == path and li == 0:
                continue
            if not curves:
                continue
            print(f"{name} layer {li}: header diff vs ref: {diff(base[0], lh)}")
            for ci, c in enumerate(curves):
                bc = base[1][min(ci, len(base[1]) - 1)]
                print(f"  curve {ci} diff vs ref curve: {diff(bc, c)}")
                bn = base[3][min(ci, len(base[3]) - 1)]
                print(f"  curve {ci} note: {len(notes[ci])} bytes, diff vs ref note: {diff(bn, notes[ci])}")


if __name__ == "__main__":
    main()

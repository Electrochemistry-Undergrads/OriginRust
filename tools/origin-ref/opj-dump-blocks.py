#!/usr/bin/env python3
"""Prints the raw block structure of the window section of an .opj file (our own tool).

For every window it lists the header, then for each layer the layer header, the annotations
(header and three blocks), the column records and the three groups of axis blocks. A block is
printed as its size and the non-zero bytes as `offset:hex` runs, so that two files (one written
by Origin, one by Sheaf) can be compared with a plain text diff.

Usage: opj-dump-blocks.py FILE.opj [--window NAME] [--full]
"""
import struct
import sys


class Reader:
    def __init__(self, data):
        self.d = data
        self.p = 0

    def line(self):
        e = self.d.index(b"\n", self.p)
        s = self.d[self.p:e]
        self.p = e + 1
        return s

    def block(self):
        n = struct.unpack_from("<I", self.d, self.p)[0]
        assert self.d[self.p + 4] == 0x0A, f"bad block at {self.p}"
        self.p += 5
        if n == 0:
            return b""
        b = self.d[self.p:self.p + n]
        self.p += n
        assert self.d[self.p] == 0x0A, f"bad block end at {self.p}"
        self.p += 1
        return b

    def raw(self, n):
        b = self.d[self.p:self.p + n]
        self.p += n
        assert self.d[self.p] == 0x0A
        self.p += 1
        return b


def runs(b, full=False):
    """Non-zero bytes as offset:hex runs (a gap of up to 3 zero bytes stays inside a run)."""
    if full:
        return b.hex()
    out = []
    i = 0
    n = len(b)
    while i < n:
        if b[i] == 0:
            i += 1
            continue
        j = i
        k = i
        while k < n and k - j <= 3:
            if b[k] != 0:
                j = k
            k += 1
        out.append(f"{i:#x}:{b[i:j + 1].hex()}")
        i = j + 1
    return " ".join(out)


def main():
    args = sys.argv[1:]
    full = "--full" in args
    only = None
    if "--window" in args:
        only = args[args.index("--window") + 1]
    path = [a for a in args if not a.startswith("--") and a != only][0]
    r = Reader(open(path, "rb").read())
    r.line()
    r.block()
    r.block()
    nsets = 0
    while True:
        h = r.block()
        if not h:
            break
        r.block()
        r.block()
        nsets += 1
    print(f"data sets: {nsets}")
    while True:
        wh = r.block()
        if not wh:
            break
        name = wh[2:27].split(b"\0")[0].decode("latin1")
        show = only is None or only == name
        if show:
            print(f"WINDOW {name} header({len(wh)}): {runs(wh, full)}")
        li = 0
        while True:
            lh = r.block()
            if not lh:
                break
            if show:
                print(f" LAYER {li} header({len(lh)}): {runs(lh, full)}")
            ai = 0
            while True:
                ah = r.block()
                if not ah:
                    break
                b1 = r.block()
                b2 = r.block()
                grp = len(b1) in (0x5E, 0x0A) and len(b2) == 4
                if grp:
                    n = struct.unpack("<I", b2)[0]
                    if show:
                        print(f"  ANN {ai} (group) header({len(ah)}): {runs(ah, full)}")
                        print(f"     b1({len(b1)}): {runs(b1, full)}   b2: size {n}")
                    mh = r.raw(n)
                    while True:
                        if show:
                            print(f"   member header({len(mh)}): {runs(mh, full)}")
                        c1 = r.block()
                        c2 = r.block()
                        if len(c1) in (0x5E, 0x0A) and len(c2) == 4:
                            if show:
                                print(f"     b1({len(c1)}): {runs(c1, full)}   b2: size")
                            mh = r.raw(struct.unpack("<I", c2)[0])
                            continue
                        c3 = r.block()
                        if show:
                            print(f"     b1({len(c1)}): {runs(c1, full)}\n     b2({len(c2)}): {c2!r}\n     b3({len(c3)}): {runs(c3, full)}")
                        break
                else:
                    b3 = r.block()
                    if show:
                        print(f"  ANN {ai} header({len(ah)}): {runs(ah, full)}")
                        print(f"     b1({len(b1)}): {runs(b1, full)}")
                        print(f"     b2({len(b2)}): {b2!r}")
                        print(f"     b3({len(b3)}): {runs(b3, full)}")
                ai += 1
            ci = 0
            while True:
                rec = r.block()
                if not rec:
                    break
                note = r.block()
                if show:
                    print(f"  COL {ci} record({len(rec)}): {runs(rec, full)}")
                    if note:
                        print(f"     note: {note!r}")
                ci += 1
            extra = r.block()
            if show:
                print(f"  extra block({len(extra)}): {runs(extra, full)}")
            for g in range(3):
                k = 0
                while True:
                    b = r.block()
                    if not b:
                        break
                    if show:
                        print(f"  AXIS g{g} b{k} ({len(b)}): {runs(b, full)}")
                    k += 1
            li += 1
    print(f"end of windows at {r.p} of {len(r.d)}")


if __name__ == "__main__":
    main()

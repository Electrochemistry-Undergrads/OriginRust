#!/usr/bin/env python3
"""Regenerates the synthetic Excel files of crates/io/tests/fixtures/excel/ (T30).

    python3 -m venv /tmp/venv && /tmp/venv/bin/pip install openpyxl xlwt
    /tmp/venv/bin/python tools/make-excel-fixtures.py [OUTPUT_DIR]

Every file is made up (no lab data) and below 10 kB. The Rust tests in
crates/io/src/excel/tests.rs state what each file must read as; this script is the only place
that knows how the files are built. Files the Rust writer can make itself are not here.
"""

import datetime as dt
import re
import sys
import zipfile
from pathlib import Path

import openpyxl
import xlwt
from openpyxl.chart import BarChart, Reference
from openpyxl.utils.datetime import CALENDAR_MAC_1904

OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent / "crates/io/tests/fixtures/excel"
FIXED = dt.datetime(2026, 1, 1)


def new_book():
    wb = openpyxl.Workbook()
    wb.properties.created = FIXED
    wb.properties.modified = FIXED
    wb.properties.creator = "sheaf-fixtures"
    return wb


def save(wb, name):
    wb.save(OUT / name)


def patch_cached(name, sheet_file, values):
    """Gives formula cells a cached value, as Excel stores it (openpyxl writes none)."""
    path = OUT / name
    with zipfile.ZipFile(path) as z:
        items = [(i, z.read(i.filename)) for i in z.infolist()]
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for info, data in items:
            if info.filename == sheet_file:
                text = data.decode("utf-8")
                for ref, value in values.items():
                    text, n = re.subn(rf'(<c r="{ref}"[^>]*><f>[^<]*</f>)<v\s*/>', rf"\g<1><v>{value}</v>", text)
                    if n == 0:
                        text, n = re.subn(rf'(<c r="{ref}"[^>]*><f>[^<]*</f>)<v></v>', rf"\g<1><v>{value}</v>", text)
                    assert n == 1, (ref, text[:600])
                data = text.encode("utf-8")
            z.writestr(info.filename, data)


def basic():
    """Three label rows, numbers, text, ints, date-times, and formulas with and without a cached value."""
    wb = new_book()
    ws = wb.active
    ws.title = "Data"
    ws.append(["Potential", "Current", "Sample", "When", "Count"])
    ws.append(["V", "A", None, None, None])
    ws.append(["scan 1", "scan 1", None, None, None])
    for i in range(5):
        ws.append([-0.2 + 0.1 * i, (i + 1) * 1.5e-6, "abcde"[i] * 2, dt.datetime(2026, 11, 5 + i, 12, 0, 0), i + 1])
    for r in range(4, 9):
        ws.cell(r, 4).number_format = "yyyy-mm-dd hh:mm:ss"
    cached = wb.create_sheet("Cached")
    cached.append(["x", "double"])
    for i in range(1, 5):
        cached.append([i, f"=A{i + 1}*2"])
    plain = wb.create_sheet("Uncached")
    plain.append(["x", "triple"])
    for i in range(1, 5):
        plain.append([i, f"=A{i + 1}*3"])
    save(wb, "basic.xlsx")
    patch_cached("basic.xlsx", "xl/worksheets/sheet2.xml", {"B2": 2, "B3": 4, "B4": 6, "B5": 8})


def legacy():
    """The old binary format: label rows, numbers, text, a date column, a second sheet."""
    wb = xlwt.Workbook()
    ws = wb.add_sheet("Old")
    date = xlwt.easyxf(num_format_str="yyyy-mm-dd")
    for c, text in enumerate(["Time", "Signal", "Tag", "Day"]):
        ws.write(0, c, text)
    for c, text in enumerate(["s", "mV", "", ""]):
        if text:
            ws.write(1, c, text)
    for i in range(4):
        ws.write(2 + i, 0, float(i))
        ws.write(2 + i, 1, 10.5 + i)
        ws.write(2 + i, 2, f"t{i}")
        ws.write(2 + i, 3, dt.datetime(2020, 2, 26 + i), date)
    second = wb.add_sheet("Second")
    for i in range(3):
        second.write(i, 0, i + 1)
        second.write(i, 1, (i + 1) ** 2)
    wb.save(str(OUT / "legacy.xls"))


def date1904():
    """The 1904 date system (old Mac files): a date-time, a time of day and a duration column."""
    wb = new_book()
    wb.epoch = CALENDAR_MAC_1904
    ws = wb.active
    ws.title = "D"
    ws.append(["When", "Clock", "Elapsed"])
    for i in range(3):
        ws.append([dt.datetime(2020, 1, 1 + i, 12, 0, 0), dt.time(13, 45, 10 + i), 1.5 + i])
    for r in range(2, 5):
        ws.cell(r, 1).number_format = "yyyy-mm-dd hh:mm:ss"
        ws.cell(r, 2).number_format = "hh:mm:ss"
        ws.cell(r, 3).number_format = "[hh]:mm:ss"
    save(wb, "date1904.xlsx")


def leap1900():
    """Serial numbers around Excel's non-existent 1900-02-29."""
    wb = new_book()
    ws = wb.active
    ws.title = "Leap"
    ws.append(["Day"])
    for serial in (1, 59, 60, 61, 25569, 45000):
        ws.append([serial])
    for r in range(2, 8):
        ws.cell(r, 1).number_format = "dd/mm/yyyy"
    save(wb, "leap1900.xlsx")


def merged():
    """A merged title row over the header rows, and a merged block inside the data."""
    wb = new_book()
    ws = wb.active
    ws.title = "Merged"
    ws.append(["Run 7", None, None])
    ws.append(["X", "Y", "Z"])
    ws.append(["s", "V", "A"])
    for i in range(4):
        ws.append([i, i * 2.0, i * 3.0])
    ws.merge_cells("A1:C1")
    ws["B9"] = 9.0
    ws["B10"] = None
    ws["A9"] = 8
    ws["A10"] = 8
    ws["C9"] = 7.0
    ws["C10"] = 7.0
    ws.merge_cells("B9:B10")
    save(wb, "merged.xlsx")


def hidden():
    wb = new_book()
    ws = wb.active
    ws.title = "Visible"
    ws.append(["a"])
    ws.append([1])
    ws.append([2])
    ws.append([3])
    for name, state in (("Hidden", "hidden"), ("Secret", "veryHidden")):
        s = wb.create_sheet(name)
        s.append(["b"])
        s.append([10])
        s.append([20])
        s.append([30])
        s.sheet_state = state
    save(wb, "hidden.xlsx")


def charts():
    """Data on one sheet, a chart on a worksheet without cells, and a sheet that is only a chart."""
    wb = new_book()
    ws = wb.active
    ws.title = "Data"
    ws.append(["x", "y"])
    for i in range(1, 5):
        ws.append([i, i * i])
    chart = BarChart()
    chart.add_data(Reference(ws, min_col=2, min_row=1, max_row=5), titles_from_data=True)
    only = wb.create_sheet("OnlyChart")
    only.add_chart(chart, "B2")
    sheet = wb.create_chartsheet("ChartSheet")
    other = BarChart()
    other.add_data(Reference(ws, min_col=2, min_row=1, max_row=5), titles_from_data=True)
    sheet.add_chart(other)
    save(wb, "charts.xlsx")


def mixed():
    """Text among numbers, booleans, error cells, dates among text, large integers."""
    wb = new_book()
    ws = wb.active
    ws.title = "Mixed"
    ws.append(["conc", "name", "flag", "ratio", "stamp", "big"])
    rows = [
        [1.5, "red", True, 1.0, dt.datetime(2026, 3, 1), 9007199254740992],
        ["n.d.", "green", False, "=1/0", "not a date", 12345678901],
        [3.5, "blue", True, 4.0, dt.datetime(2026, 3, 3), 7],
        ["<LOD", "red", False, 8.0, dt.datetime(2026, 3, 4), 8],
    ]
    for r in rows:
        ws.append(r)
    ws["D3"].value = "#DIV/0!"
    ws["D3"].data_type = "e"
    for r in range(2, 6):
        ws.cell(r, 5).number_format = "yyyy-mm-dd"
    save(wb, "mixed.xlsx")


def leading():
    """Data that starts at C4 below empty rows and to the right of empty columns."""
    wb = new_book()
    ws = wb.active
    ws.title = "Offset"
    ws["C4"], ws["D4"] = "Time", "Value"
    ws["C5"], ws["D5"] = "s", "mV"
    for i in range(4):
        ws.cell(6 + i, 3, i * 0.5)
        ws.cell(6 + i, 4, 100 + i)
    save(wb, "leading.xlsx")


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    for fn in (basic, legacy, date1904, leap1900, merged, hidden, charts, mixed, leading):
        fn()
    for p in sorted(OUT.iterdir()):
        size = p.stat().st_size
        print(f"{p.name}: {size} bytes")
        assert size < 10_000, p

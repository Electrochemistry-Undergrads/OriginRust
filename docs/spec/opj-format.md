# Origin project file format (`.opj`)

Status: written while building `crates/io/src/opj/`. Covers what the reader needs: workbooks,
sheets, columns and their values (checked cell by cell against Origin, section 12) and basic graphs
(section 10). Matrices and notes are only sketched. Section 13 describes what the writer does and
what Origin 2016 needed to open a written file.

Provenance tags used on statements:

- **[obs]** seen directly in project files (the five projects written by Origin 2016 /
  version 9.3, and the 50 sample projects that ship with Origin 2016, which were saved by
  versions 7.0 to 9.3).
- **[pub]** taken from public documentation of the format (the `liborigin` project and its
  notes), re-derived and then checked against the files where possible. Where the files
  disagreed with the documentation, the files are followed and the difference is stated.
- **[inf]** inferred from patterns; not confirmed by a second source.

This document contains no data values or names from private files. Example names are
invented (`Book1`, `A`) or are Origin's own conventions.

## 1. Summary

An `.opj` file is a single stream with no index and no compression [obs]:

1. one text line, the signature (`CPYA 4.3168 226 W64 #`);
2. a series of **blocks**, each a 4-byte little-endian size, a newline, the body, a newline;
3. in that block stream, in this order: a version header, the **data sets** (one per column of
   every workbook: a header block, a data block, a mask block), then the **windows**
   (workbooks, graphs, matrices, ...), each with **layers** (= sheets for a workbook) that carry
   the **column records**;
4. after the windows: a list of named parameters, note windows, the project folder tree and
   some blobs the reader does not use.

Values live in the data sets; names, long names, units, comments, the sheet structure and
the column role (X, Y, ...) live in the windows. A column record in a layer points to its data
set by position in the data-set list. A data set is named `<window>_<column>`, with `@<n>`
appended for sheets other than the first, which gives a second way to find the link [obs].

Every column of an Origin 2016 file is stored as a 10-byte-or-wider "text or number" cell,
whether or not it holds text; older versions store plain 8-byte doubles [obs]. "No value" in a
double is the number -1.23456789e-300 [obs]. Text is Windows-1252 (an ANSI code page) [inf].

## 2. Container layout

### 2.1 Signature line [obs]

`CPYA <major>.<build> <n> [W32|W64] #` followed by a newline. Examples seen: `CPYA 4.3168 226 W64 #`
(Origin 2016), `CPYA 4.2889 890#` (version 8.1, no platform field), `CPYA 4.3165 195 W64 #`
and one file whose first number reads `4,3165` (a decimal comma written by a localised
program). The second number is the build counter of the program that saved the file. The
platform field is present from build 4.2979 on. The Unicode project format (`.opju`) starts
with `CPYUA` [pub] and is not covered here.

### 2.2 Blocks [obs]

```
+----------------+------+-------------------+------+
| size: u32 LE   | 0x0A | body: `size` bytes| 0x0A |
+----------------+------+-------------------+------+
```

- A block of size 0 has **no body and no second newline**: it is exactly 5 bytes
  (`00 00 00 00 0A`). Empty blocks are used everywhere as list terminators and as "nothing here".
- Both newline bytes are always present in the files seen, so a reader can check them and
  detect damage immediately.
- Public documentation describes the size as ASCII decimal text; in all 55 files it is a 4-byte
  binary integer, so that description does not apply to these versions.
- One exception to the framing: the first member of a grouped annotation has its header written
  as `n` raw bytes plus a newline with **no size field** (section 5.2).

### 2.3 Version header [obs]

The first block after the signature line is the header. Its size by file version: 39 (versions
7.0 to 8.02), 75 (8.09 to 8.1), 107 (8.11 to 8.5), 111 (8.6 to 9.3). At body offset `0x1B` there is a
little-endian double with the Origin version (`9.3`, `8.11`, `7.58`, ...) [obs][pub: the
documentation derives a build number as `trunc(v*100)` for v above 8.5]. The header block is followed by
one empty block.

### 2.4 Order of sections [obs]

```
signature line
header block, empty block
data sets:   { header block (non-empty), data block, mask block }*   then an empty block
windows:     { window ... }*                                          then an empty block
parameters:  { name line, 8-byte double, newline }*  then a line "\0\n"
notes, project tree, other blobs
```

All 55 files walked cleanly with this layout from the first block to the empty block that
ends the window list; this is a strong check on the framing rules and the window structure in
section 4.

## 3. Data sets

### 3.1 Header block [obs, except where tagged]

Header size by file version: 131 (7.x to 8.02), 140 (8.09 to 8.6), 146 (9.0 to 9.1), 147 (9.2 and
later). Fields at fixed offsets (the same in all versions seen):

| offset | size | meaning |
|---|---|---|
| `0x16` | u16 LE | type code, see 3.3 |
| `0x19` | u32 | number of rows the header claims. Equals data size / cell size for every file seen except data sets whose data block is empty (data not stored), where the header still claims the rows |
| `0x1D`, `0x21` | u32, u32 | first and last used row [pub]; `0` and rows for files seen |
| `0x3D` | u8 | bytes per cell. `0` is treated as 8 [pub] |
| `0x3F` | u8 | flags; bit `0x08` marks unsigned integers [pub]. Other values (1, 16, 32, 48) occur and have no known effect on the values |
| `0x58` | 25 bytes | data-set name, NUL padded, Windows-1252 |

The name is `<window name>_<column short name>`, optionally followed by `@<n>`. The number after
`@` is a sheet counter, **not** the position of the sheet in the workbook: a workbook whose
sheets are named by counters 6, 7, 8 occurs [obs]. Window names may contain underscores; the
column name is whatever follows the window name and an underscore [inf].
Function data sets and matrix sheets have other names (matrices have no underscore) [obs].

### 3.2 Data block and mask block [obs]

The data block holds `rows x cell size` bytes with no framing inside. The number of rows is the
block size divided by the cell size; trailing bytes that do not make a whole cell were never
seen and are ignored. A data set with an empty data block exists (it belongs to an unfilled
column). The mask block was empty in every file seen; public documentation says it is read but
its meaning is not documented (so nothing is decoded from it) [pub][obs].

Columns are stored padded: in the small private files a worksheet with six numbers still stored 32
cells per column, the rest being "no value" cells [obs]. The reader keeps the padding (it cannot tell
padding from data that is really missing).

### 3.3 Value types [obs, with names from pub]

The type code is a 16-bit number. Reading it:

- Bit `0x0100` set: **text-or-number cells** (section 3.4). Seen in every column of Origin 2016
  files and in many 8.x files. The other bits then carry display details (`0x5121`, `0x6121`,
  `0x4121`, `0x0121`, `0x6120`, `0x6130`) that did not matter for the values.
- Else bit `0x0800` set: integer of the cell size: 1 byte (`0x6821`), 2 bytes (`0x6803`),
  4 bytes (`0x6801` [pub]); signed unless the unsigned flag is set. Seen for 1-byte and 2-byte
  data in matrices [obs]; 4-byte integers only from documentation.
- Else low byte of the form `0x2x` (`0x6021`, `0x5021`, `0x4021`, `0x6020`): **fixed-width text**,
  `cell size` bytes per row (28, 50 seen), NUL padded [obs].
- Else low byte below `0x10` and 8 bytes per cell (`0x6001`, `0x5001`, `0x1001`, `0x4001`, `0x6000`,
  `0x0001`, `0x0401`): little-endian IEEE double [obs]. The high nibble varies with the role the
  column had when it was made and does not change the layout.
- Else low byte below `0x10` and 4 bytes per cell (`0x6003`): float [pub], not seen.
- 16 bytes per cell (`0x0201`, `0x6201`, `0x0162`): complex numbers, two doubles [inf]; not decoded.
- `0x6081`, `0x60B1`, `0x60A4`, `0x60A5`: function definitions and other generated data [obs];
  the data block holds the formula text, not a column. Not decoded.

### 3.4 Text-or-number cells [obs]

Each row is `cell size` bytes (at least 10; 10 to 171 seen). Byte 0 is the kind: `0` for a number,
`1` for text. Byte 1 was `0` for text cells and nearly always for numbers. A few hundred number
cells in the sample files have another value in byte 1 (for example `0x40`, `0x50`..`0x65`); they
decode as ordinary numbers and the meaning of the byte is unknown. For a number, bytes 2..10 are a
little-endian double. For text, bytes 2.. up to `cell size` are the text, NUL terminated or padded.
The cell size is chosen per column to fit its longest text (`longest + 2`), so no text was seen to
overflow.

### 3.5 Missing values [obs]

- In doubles (plain or inside a text-or-number cell) the missing value is the double
  `-1.23456789e-300` (bit pattern `0x81AA74FE1C132C0E`, bytes `0E 2C 13 1C FE 74 AA 81` in the file; the reader compares
  bit for bit). It occurs in all projects: from
  0.3 percent to about 60 percent of the cells of the five private projects.
- NaN was never found in any data block.
- In text cells the empty string is the missing value (a text cell of kind 1 with no characters).
  The reader treats such a cell in an otherwise numeric column as a missing number.
- Integers have no missing marker.

### 3.6 Date and time columns [obs]

Dates and times are stored as doubles (Origin day numbers); only the column record says that
they are dates (value-type byte `0x02` time, `0x03`/`0x33` date, `0x04`/`0x34` month,
`0x05`/`0x35` weekday [pub]). The reader returns the raw numbers.

## 4. Windows

After the empty block that ends the data sets, windows follow until an empty block [obs].
One window is:

```
window header block
{ layer }*          ends with an empty block where a layer header would be
```

A **layer** (a sheet, for a workbook) is:

```
layer header block              (non-empty; an empty block ends the layer list)
{ annotation }*                 ends with an empty block where an annotation header would be
{ column record block, note block }*      ends with an empty column record block
one extra block                 (always empty in the files seen) [inf]
three groups of axis blocks     each group is blocks until an empty block; empty for workbooks
```

This structure was checked by walking all 55 files to the end of the window list [obs]. Which
window is a workbook is not stated in the window header in any way that was identified
(sizes 195, 244, 280, 343, 346, 552, 634 all occur). The reader decides from names:
a window is a workbook if some data set is named `<window name>_...` [inf]. Graph and matrix
windows have no such data sets. Which windows are graphs is decided from the layer header, see 10.0.

### 4.1 Window header (`0x02`..) [obs]

| offset | size | meaning |
|---|---|---|
| `0x02` | 25 bytes | window short name, NUL padded |
| `0x1B` | 4 x i16 | window rectangle (left, top, right, bottom) [pub] |
| `0x32` | u8 | state bits: 1 minimised, 2 maximised [pub] |
| `0x45` | up to 20 bytes | template name (`ORIGIN` for workbooks) [obs] |
| `0xC3` | to end | label: the long name. Plain text up to the first `@${`, which starts storage data. May hold several lines (a comment after the long name); the reader keeps the first line [obs] |

For header sizes of 195 and less the label is absent.

### 4.2 Layer header (361 bytes in 9.x) [obs]

Offset `0x02`: bit 0 is set in graph layers and clear in workbook and matrix layers (section 10.0).

Offset `0xD2`: 32 bytes, NUL padded, the sheet name. It is preceded at `0xD0` by two bytes that read
`Pd` (workbook sheets) or `PdM` (matrix sheets, `M` as the first letter of the name) in the files
seen; the meaning of these bytes is not known. Other fields (grid, view) were not examined for
workbooks. For graph layers public documentation lists axis ranges and ticks at `0x0F`..`0x6B`
[pub]; not used.

### 4.3 Column record (515 bytes in 9.x, 489 in 8.02) [obs]

| offset | size | meaning |
|---|---|---|
| `0x04` | u16 | position (1-based) of the column's data set in the data-set list |
| `0x11` | u8 | role: 0 = Y, 3 = X, 5 = Z, 6 = X error, 2 = Y error, 4 = label; 1 and other values = disregard [pub][obs]. Value 1 occurs often and was mapped to "disregard" without a second source [inf]. Value 96 appears in graph curves, never in workbooks |
| `0x12` | 12 bytes | short name, NUL padded. Cut at 11 characters in some files (`Independent1` appeared as `Independent`); the data-set name is the full name and is preferred |
| `0x1E` | u8 | value type (9 plain numeric, 0, 0x10, 0x08 numeric, `0x02` time, `0x03`/`0x33` date, `0x31` text, ...) [pub][obs] |
| `0x1F` | u8 | display precision: `>= 0x80` significant digits, else decimals offset by 3 [pub]; not used |
| `0x4A` | u16 | column width in tenths of characters [pub]; not used |

The column record is followed by its **note block** (section 4.4). The same 515-byte record is
used for the curves of a graph (a curve has the role byte 96, and the Y data set index at
`0x04`, the X data set index at `0x23` [pub]).

For all 5065 worksheet column records in the sample files and private projects, 99.5 percent
of the data set indices at `0x04` pointed to a data set whose name matched `<window>_<column>`;
the rest were function and matrix data.

### 4.4 Column note: long name, units, comments [obs]

The block that follows a column record holds text in Windows-1252:

```
long name CR LF units CR LF comments   [ "@${" storage ]
```

Everything from the first `@${` on is machine data (an embedded record such as
`@${[0|3|SYSTEM|32|385469799]}[44]...`, parameters, import information, script) and is
ignored. The text before it has up to three parts separated by CR LF; the third part may itself
contain CR LF. Parts that are absent are empty. Seen: `name` only; `name CRLF units`;
`name CRLF units CRLF comment`; `CRLF CRLF comment` (empty long name and units). A block of one NUL
byte or of size 0 means no labels. The assignment of the three parts (long name, units, comments)
matches Origin's header rows (Long Name, Units, Comments) [inf].
Origin's text escapes (for example `\g(a)` for a Greek letter and `\+(0)` for a superscript) are not
decoded and stay in the strings.

Other label rows (parameters, user-defined rows) and the "set column values" formula are also in
this note storage and in the annotations; they are not decoded [unknown].

### 4.5 Sheets and sheet names [obs]

Each layer of a workbook window is a sheet, in order. Sheet names: layer header offset `0xD2`. The
data sets of a sheet carry a `@<n>` suffix for every sheet but, usually, the first; `n` is not the
sheet index (section 3.1), so the link must be made through the column records. The column order
within a sheet is the order of its column records, not the data-set order [obs].

## 5. Annotations and other layer contents

### 5.1 Annotation [obs]

An annotation is a header block (123 or 131 bytes), then block 1, block 2 and block 3. The reader
steps over them. In a workbook sheet the annotations hold the sheet's embedded storage and the
"set column values" formulas; in a graph they hold text labels, lines, legends and colour scales [pub].
In graphs a name at header offset `0x46` (`Legend`, `XB`, `PL`, `ZCOLORS`) says which one it is [pub];
not used here. A layer has zero or more annotations (0 to 34 seen), closed by an empty block.

### 5.2 Grouped annotations [obs]

If block 1 has size `0x5E` or `0x0A` and block 2 has size 4, the annotation is a **group** [pub]. Block 2 holds the
size `n` (little-endian) of the first member's header. That header follows as `n` raw bytes plus a newline,
without a size field, and the member's own block 1, block 2 (and block 3, unless it is a group too)
follow. The rule applies recursively, which the reader does in a loop. Seen in graph windows
containing legends or text groups (123-byte headers); not seen in the workbooks of the private files.
Misreading this rule breaks the window walk, which is how it was found (one of Origin's sample
files).

## 6. Sections after the windows

[obs] unless marked.

- **Parameters.** Lines of `name`, newline, an 8-byte double, newline, up to a line that is a single NUL (`00 0A`).
  One entry was seen in a small file (a name of 6 letters with value 0). The public docs describe the same
  loop, whose entries are project-wide parameters.
- **Notes.** Each note window has a header block (80 bytes), a name block (NUL-terminated, `ResultsLog`
  is the results log window that every project has), and a text block. Then an empty block.
- **Project tree.** A few blocks with the project name and folders (names as NUL-terminated strings) and, per entry,
  16-byte and 32-byte records that appear to hold timestamps as doubles [inf].
- **Further blobs.** High-entropy data at the end of the file (embedded storage, compressed). Not understood.

The reader stops after the window list.

## 7. Text encoding [inf]

All text in the files read is ASCII or Windows-1252 (the code page of the machine that saved the
file). The files do not say which code page was used. The only non-ASCII bytes found in column notes of all 55
files were `0xB0` (degree sign) and `0xB5` (micro sign), which agree between Windows-1252 and Latin-1; the five
private projects have none. The reader decodes Windows-1252. Files saved on a machine with another ANSI code page (for example Japanese) would decode
wrongly.

## 8. Version differences actually seen

| file version | header block | data-set header | column record | notes |
|---|---|---|---|---|
| 7.0x - 8.02 | 39 | 131 | 489 (8.02) | plain 8-byte doubles |
| 8.09 - 8.1 | 75 | 140 | | text-or-number cells (10 bytes) seen from 8.09 |
| 8.11 - 8.6 | 107 (8.11 - 8.5), 111 (8.6) | 140 | | |
| 9.0 - 9.1 | 111 | 146 | | platform tag `W32`/`W64` appears in the signature from build 4.2979 (8.6) |
| 9.2 - 9.3 (Origin 2016) | 111 | 147 | 515 | worksheet columns are text-or-number cells (cell size >= 10); plain doubles still occur for generated data |

Everything else described here held across all of these versions.

## 9. What a reader does

1. Read the signature line; require `CPYA`.
2. Read the header block (version at `0x1B`) and the following empty block.
3. Read data-set headers, step over data and mask blocks, remembering where each data block lies;
   stop at the empty block.
4. Read windows as in section 4, stepping over annotations and axis blocks.
5. For every window whose name is a prefix of data-set names, for every layer, for every column record:
   find the data set by index (and check that its name starts with `<window>_`), read and decode its
   data block, split the note into labels, map the role byte.
6. If the window list cannot be read, group the data sets by name into workbooks and sheets (sheet number
   from the `@n` suffix), the first column as X, the others as Y.

## 10. Graphs [obs unless marked]

Graph windows are told apart as described in 10.0. `crates/io/src/opj/graph.rs` turns them into
`sheaf_core::graph::Graph`.

**Version coverage.** Every offset in this section was found and checked on files written by Origin
9.x (the owner's projects and graphs built in Origin 2016). Projects saved by Origin 7.0 to 8.6 have
the same block structure, and the rule in 10.0 was checked against them, but the offsets of the page
size, frame, axes and curve records were *not* checked on any pre-9 file. For a file older than 9.0
the reader still imports the graphs that 10.0 identifies, and always adds a warning that graphs of
such a file are not verified and may be missing or wrong (also when no graph was read, since a
missed graph would otherwise go unmentioned). Offsets below were found by building graphs in Origin 2016 with known settings
(`tools/origin-ref/opj-make-test-graphs.ps1`, and one-property-at-a-time variations whose records
were compared byte by byte), by comparing with the graph in the owner's figure project, and with the
public description of the format [pub]. All numbers are little-endian.

### 10.0 Which windows are graphs

Rule used by the reader: a window is a graph when the layer header of its **first layer** has bit 0
(`0x01`) of the byte at offset `0x02` set, **and** that layer has axis blocks (six 515-byte blocks in
the first of the three axis groups of section 4).

Why the axis blocks alone are not enough: in projects saved by Origin 8.x and 7.x, the layers of
workbooks and matrices carry the same six axis blocks as graph layers. Origin 9.x writes none for
them. A reader that went by the blocks read every older workbook a second time as a bogus graph (its
columns became "plots" of type 0, with a tiny frame and the default page).

Evidence for the bit (offset `0x02` of the layer header is the low byte of a flag word; the values
seen are `0x1F`/`0x5F` for graph layers, `0x5E`/`0x7E` for workbook sheets and `0xDE` for matrices) [obs]:

- Over the 54 sample projects of Origin 2016, the five owner projects and 634 windows in all, the
  bit is clear in every layer of all 360 windows that are workbooks by the name rule of section 4,
  and set in every layer of every other window that has axis blocks except two; those two (one in
  `2D Bin and Fit.opj`, one in `Calling Fortran.OPJ`, both Origin 8.x) have the flag word `0xDE`, and
  Origin lists both as matrices.
- Cross-check against Origin 2016 itself, on 33 projects (23 saved by versions 7.0 to 8.6, 10 by
  9.x including the five owner projects): `tools/origin-ref/opj-list-windows.ps1` opens a copy of each
  in a hidden Origin and lists the windows of `doc -e W` (workbooks), `doc -e P` (graphs) and
  `doc -e M` (matrices). Before the fix the reader named 37 graph windows that Origin does not list
  as graphs (workbooks and matrices of 8.x projects, for example `Book1` in `Simple XY.opj`, which
  has one workbook and one graph). After the fix it names none: every graph window the reader finds
  is a window Origin lists as a graph.
- The rule does not change anything for 9.x projects: in all 9.x files seen, every window with axis
  blocks has the bit set (the reader's graph count for the five owner projects is the same before
  and after: 26, 1, 7, 8 and 17).

What the rule does not do: it does not find every graph. Origin lists graph windows that the
reader does not read, mostly because their layers have no axis blocks (39 windows of the 9.x files
have the bit and no axis blocks). Examples by name in the files above: `FitLine`, `ResIndp`, `Radar`
and many windows called `sparkline...`. Why the reader misses each of them was not looked into. That
gap was there before and is unchanged. The bit's meaning ("this layer is a graph layer") is inferred from the correlation above;
no second source states it [inf].

### 10.1 Window header (graphs)

| offset | size | meaning |
|---|---|---|
| `0x23` | u16 | page width in 1/600 inch (6432 for a 10.72 inch wide page) |
| `0x25` | u16 | page height in 1/600 inch |

The page units were found by exporting a graph at Origin's default 300 dots per inch: a page of
6432 units came out 3216 pixels wide.

### 10.2 Layer header (361 bytes)

| offset | size | meaning |
|---|---|---|
| `0x0F`, `0x17`, `0x1F` | 3 x f64 | X axis start, end and major step |
| `0x37` | u8 | X minor ticks (1 on a linear axis, 8 on a logarithmic one) |
| `0x38` | u8 | X scale type: 0 linear, 1 base-10 logarithm (others [pub]: 2 probability, 3 probit, 4 reciprocal, 5 offset reciprocal, 6 logit, 7 natural log, 8 base-2 log) |
| `0x3A`, `0x42`, `0x4A` | 3 x f64 | Y axis start, end and major step |
| `0x62`, `0x63` | u8, u8 | Y minor ticks, Y scale type |
| `0x71` | 4 x u16 | layer frame: left, top, right, bottom, in the page units of 10.1 |

LabTalk numbers scale types from 1 (`layer.x.type`: 1 linear, 2 log10), the file from 0. The ranges
and steps equal what `layer.x.from`, `.to` and `.inc` report (checked on four known graphs and the
owner's graph). The frame equals `layer.left`, `layer.top`, `layer.width` and `layer.height`
(percent of the page) to the rounding of those numbers.

### 10.3 Axis blocks

Group 0 is X, group 1 is Y, group 2 is Z. In each group the blocks are, by position:
0 minor grid, 1 major grid, 2 tick labels, 3 axis line and ticks, 4 title (not used), 5 the
opposite axis.

- Block 2, byte `0x15`: tick label font size in points. Byte `0x26`: bit `0x40` set means the
  labels are shown (cleared by `layer.x.showLabels = 0`).
- Block 3, u16 at `0x15`: axis line width in 1/500 point (1500 is 3 points). Byte `0x25`: bits 4 and
  5 give the major tick direction, bits 6 and 7 the minor one: 0 none, 1 in, 2 out, 3 both (a default
  Origin graph has `0xA0`, both out; the owner's Y axis has `0x50`, both in, which matches his figure).
  The meaning of 1 and 3 was inferred from these two cases, not from a rendering of each.
- Block 5, byte `0x26`: bit `0x40` set means the opposite axis line is drawn (the closed box). Set by
  `layer.x.showAxes = 3`.

### 10.4 Annotations of a graph layer

Header: the rectangle as four u16 (left, top, right, bottom, page units) at `0x03`, the object
name at `0x46`. Block 1 of a text object has 102 bytes: u16 at `0x02` is the angle in tenths of a
degree (900 for a rotated Y title), u16 at `0x04` the font size in points. Block 2 is the text with
Origin's escapes. Names seen: `XB` and `YL` (bottom and left axis title; the text `%(?X)` and
`%(?Y)` asks for the title built from the plotted column), `Legend`, `Text`, `Text1`, ... (free
text), and internal objects whose names start with an underscore or that have another block 1 size.
`\p89(...)` wraps text at 89 percent size; the reader drops that wrapper and keeps `\b(...)`,
`\i(...)`, `\+(...)`, `\-(...)` and `\g(...)`. A text of several lines is separated by CR LF.

### 10.5 Curve records

The column records of a graph layer are its curves (515 bytes, the same size as worksheet columns).

| offset | size | meaning |
|---|---|---|
| `0x04` | u16 | position (1-based) of the Y data set in the data-set list |
| `0x06` | u8 | probably plot group membership: bit `0x20` on the first plot of a group, bit `0x10` on the others (graphing.md 14.7; inferred from one test file and the owner's two grouped layers) |
| `0x0F` | u8 | line colour as a position in the colour list (10.6); 0 when automatic |
| `0x11` | u8 | line connection: 0 none, 1 and above a line (values 1 to 8 all drew straight lines through 8 points; the other kinds were not told apart) |
| `0x12` | u8 | line style: 0 solid, 1 dash, 2 dot, 3 dash dot, 4 dash dot dot, 5 short dash, 6 short dot, 7 short dash dot (the `set -d` values, modulo 8) |
| `0x15` | u16 | line width in 1/500 point (`set -w 1000` is 2 points) |
| `0x17` | u8 | symbol shape: 0 none, 1 square (the default); the others are guessed: 2 circle, 3 up triangle, 4 down triangle, 5 diamond |
| `0x19` | u16 | symbol size in 1/500 point (`set -z 12` is 6000). In a column plot (203) it is the bar width in percent of the X spacing times 50, that is (100 - gap) × 50: 4000 for Origin's default gap of 20, 3000 for 40, 2250 for 55 (`set -vg`); a copy is at `0x11A` (graphing.md 14.7) |
| `0x23` | u16 | position of the X data set (0 for none). For an error plot it holds the position of the Y data set of the plot the errors belong to |
| `0x4C` | u16 | plot type: 200 line, 201 scatter, 202 line and symbol, 203 column; 487 (`0x1E7`) is a Y error plot |
| `0xC2` | 4 bytes | fill colour of a column: red, green, blue, then 1 when the colour was chosen freely (the owner's bars are 0, 107, 172, 1) |
| `0x132` | 4 bytes | colour: a list position then three zero bytes, or red, green, blue, 1; `F7 FF FF FF` means "automatic" |

"Automatic" colours follow the list by plot position: the first plot black, the second red, and so
on. The reader gives each automatic plot the list colour of its position in the layer. Note that
`set -c 1 -w 1000` (several options in one call) is silently ignored by Origin 2016; each option
needs its own `set` call.

I could not make `set -k` change the drawn symbol shape in a hidden instance (the record changed, the
picture did not), so shapes other than the default square are not confirmed against a picture.

### 10.6 Colour list

The stored number is the position in this list (0 first). The RGB values were read from a PNG that
Origin exported with one thick line per colour; the printed values were within a few units of these
(for example 132 where 128 is listed), which is taken as anti-aliasing in the export:
0 black, 1 red (255,0,0), 2 green (0,255,0), 3 blue (0,0,255), 4 cyan, 5 magenta, 6 yellow,
7 dark yellow (128,128,0), 8 navy (0,0,128), 9 purple (128,0,128), 10 wine (128,0,0), 11 olive
(0,128,0), 12 dark cyan (0,128,128), 13 royal blue (about 0,0,160), 14 orange (255,128,0), 15 violet
(128,0,255), 16 pink (255,0,128), 17 white, 18 light grey (about 192), 19 grey (128), 20 light
yellow (255,255,128), 21 light cyan (128,255,255). The stored number is the `set -c` number minus one.

### 10.7 What the reader does with it, and what it leaves out

Read: page size; for each layer the frame, both axis ranges, steps, scale (linear or log10), minor
ticks, tick direction, tick label size and visibility, axis line width, the box; the titles of the
bottom and left axes with their font size; free text (one label per line, centred on the object's
rectangle with an estimated text width); a legend (present or not, and its position); plots with
their columns, type, colours, line width and style, symbol shape and size; the gap between the bars
of a layer's column plots (from its first column plot); error columns, which are attached to the
plot they belong to. A Y error plot (487) is attached; other error kinds are not.

Not read (the figure still draws, with the defaults of the template): top and right axis titles;
the Z axis; other scale types (drawn linear, with a warning); tick label font, bold and colour;
number formats; legend text (the legend shows the column names); the colour and style of grids and of
the frame; text colour; line and arrow objects; plot groups.

Known differences from Origin:

- Plots in a *group* take their colour, width and style from the first plot of the group in
  Origin; the file keeps the value that was set on each plot. After `set` on the second plot only, Origin
  showed the group's value and the reader shows the stored one.
- Column plots are drawn as bars from zero, 70 percent of the smallest X spacing wide, filled with the
  fill colour and outlined with the line colour. Origin's handling of bar border and fill colour was not
  decoded: a column plot whose `set -c` colour was changed is black in Origin and cyan outlined here.
- Horizontal bars, stacked bars, area, 3D, contour, polar, pie and box plots are not read.
- Layers that share one frame (double Y) are read layer by layer; whether the renderer draws a second
  Y axis for the second layer was not checked.
- Origin draws a frame around the legend and prints logarithmic tick labels as plain numbers where
  we print powers of ten.

Checks made: four graphs with known settings (line, scatter on log axes, line and symbol with a
moved frame, column) were saved by Origin and read back; ranges, steps, scale type, frame, titles,
line colour and width, symbol size and plot type agree with the report Origin printed, and the pictures
drawn by `sheaf export` look like Origin's PNG of the same graph except for the differences listed above.
The owner's figure project exports as a bar chart with error bars, a rotated axis title and the six
station names under the axis; Origin's own PNG of it and ours were compared by eye and agree in bar
colour, heights, error bars, axis range and tick marks inside.

## 11. Unknowns

- The mask block (always empty in these files).
- Byte 1 of a text-or-number cell when not 0; flags in the data-set header at `0x3F`.
- Whether text longer than `cell size - 2` is ever truncated or stored elsewhere.
- The meaning of designation value 1 (read as "disregard"), and of values 8 to 17, 64, 80, 82, 128, 129 seen in
  the sample files (read as "disregard").
- How a window records that it is a workbook (a flag, or only its layers). The reader uses names.
- Excel workbooks embedded in a project (layer header differs [pub]); not seen in the files used.
- The two bytes before the sheet name (`Pd` / `PdM`).
- Complex data sets (16 bytes per cell) and function data sets.
- Hidden columns, column widths, number formats and the "set column values" formula, the user parameter rows
  of a worksheet.
- Which code page non-ASCII text uses.
- The notes, project tree and the trailing blobs beyond what section 6 says.
- Whether the one extra block after the column records can be non-empty (it was empty in all 55 files).

## 12. Comparison with Origin, cell by cell

Method: `tools/origin-ref/opj-dump-cells.ps1` opens a copy of a project in a hidden Origin (through
COM) and writes, for every workbook, sheet and column, the short name, long name, units, comments,
designation, number of used rows and the cells (every row of columns with at most the limit given by
`-AllRowsLimit`, else the first three, last three and twenty spread rows).
`cargo run -p sheaf-io --example opj_dump -- FILE --cells [--all-rows LIMIT]` writes the same listing
from our reader, and `tools/origin-ref/opj-compare-cells.py` compares them (numbers bit for bit, text
exactly).

The COM collections (`WorksheetPages`, `Pages`) stay empty in a hidden instance, and `GetWorksheet`
answered for only one sheet of a test project (it returned an error number for the others). The script
therefore reads everything with LabTalk through `Execute` (`doc -e W`, `wks.colN.name$`, `cell(r,c)`,
`cell(r,c)$`). Details that cost time: Origin shows its missing value as `--` in `cell(r,c)$`;
`cell(r,c)$` past the end of a column's data makes the whole script fail (so used rows are found with
a number scan, then a bisection for the text length); LabTalk has no `while` loop that this Origin
accepts (use `for`); a variable named `a` clashes with column A; `wks.maxRows` is not valid
(`wks.nrows` is).

LabTalk designation codes: 1 Y, 2 disregard, 3 Y error, 4 X, 5 label, 6 Z, 7 X error.

| project | workbooks | sheets | columns | cells compared | result |
|---|---|---|---|---|---|
| synthetic project from `opj-make-test-project.ps1` (numbers, text, mixed text and numbers, missing values, 13 display formats, three sheets, empty and hidden books) | 5 | 7 | 35 | 423 | no difference (eight numbers in a mixed column come back as text with the same digits) |
| Final Project Figures | 1 | 1 | 3 | 18 | no difference |
| NiLMO Separation | 3 | 3 | 6 | 98 | no difference |
| XRD of all dopants for Pittcon | 10 | 10 | 84 | 175068 (every row) | no difference |
| CV Specific Capacity | 27 | 102 | 485 | 7144 (every row) | no difference |
| Integration of Delithiation SF data (243 MB) | 34 | 34 | 118 | 2820 (columns over 3000 rows sampled) | no difference |

"No difference" means window names, long names, short names, units, comments, designations, used
row counts and every listed cell agree, numbers to the last bit. Not compared: the cells that were
not sampled in the large project, hidden columns, column widths and number formats, matrices
(`doc -e M` was not run), and the "set column values" formulas.

## 13. Writing

The writer is `crates/io/src/opj/write/` (`sheaf_io::opj::write`, `write_file`). It writes the
workbooks and graphs of a project in the format of Origin 2016 (version 9.3). Graphs are 13.9; what
a graph holds that the file does not is named in a warning. Tags: **[orig]** checked by opening the written file in Origin 2016
(sections 13.7 and 13.8), **[obs]** copied from what Origin writes, **[inf]** a guess that Origin
accepted without showing a difference.

### 13.1 Order of the file [orig]

```
signature line                      CPYA 4.3168 226 W64 #
header block (111), empty block
data sets:   { header (147), data, mask (empty) } per column, all workbooks, sheets in order
             an empty block
windows:     { window header, { layer header, empty block (no annotations),
                                { column record (515), note } per column,
                                5 empty blocks } per sheet,
               an empty block (end of the layer list) } per workbook
             an empty block
parameters:  the line "\0\n"
notes:       an empty block, the results log note, an empty block
tree:        the folder tree (13.5)
records:     eight attribute records (13.6)       the file ends after the last one
```

Windows follow the order of the project. Data sets are numbered from 1 in file order; the column
record of a column holds the number of its data set. The name of a data set is
`<window>_<short name>`, with `@<n>` added for the sheets after the first, where `n` is the position
of the sheet in the workbook counted from 1 (the first sheet has no suffix).

### 13.2 Values [orig]

Every column is stored as text-or-number cells (section 3.4), whether it holds text or not, and
every column of a sheet gets the same number of rows: the longest column of the sheet, padded with
missing cells (32 rows for a sheet whose columns are all empty, which is what a new Origin sheet
has).

- A number is a cell of 10 bytes: `00 00` and the double. A missing number (NaN in our model) is the
  double -1.23456789e-300 (section 3.5). Infinities are stored as they are.
- Text is a cell of `size` bytes: `01 00`, the text in Windows-1252, padding with NUL. `size` is the
  longest text plus 3 (the two header bytes and one terminator) and at least 10, so a text column of
  short words has 10-byte cells like a number column. An empty text is written as a missing number.
  The size is a single byte, so a text is at most 252 bytes.
- Header fields written for a data set: `0x16` type word (13.3), `0x18` 3 for a column with at least one value
  and 1 for an empty one, `0x19` rows, `0x1D` position of the first cell that holds a value (0-based),
  `0x21` position after the last one (both 0 for an empty column), `0x25` and `0x2D` the double 1.0,
  `0x3D` cell size, `0x3E` the byte 0x14, `0x52` the data-set number of the column this one belongs
  to (below), `0x58` the name. A few more bytes are constant (`0x02`: 01, `0x06`: FA, `0x0A`: A0 0F 01,
  `0x71`: CA 10) [obs].
- "Belongs to" (`0x52` of the data set and `0x23` of the column record): a Y column belongs to the
  nearest X column on its left, an error, Z or label column to the nearest Y column on its left, an
  unused column (in the record only) to the nearest X column; 0 when there is none [obs from a
  project with every role; the reader does not use it, Origin loaded files with it set this way].

### 13.3 Roles and type words [orig]

| role | record byte `0x11` | type word (data set `0x16`, record `0x25`) |
|---|---|---|
| X | 3 | 0x5121 |
| Y | 0 | 0x6121 |
| Z | 5 | 0x6120 |
| Y error | 2 | 0x6130 |
| X error | 6 | 0x6130 |
| label | 4 | 0x6120 (only when the sheet has a Y column; see below) |
| unused | 1 | 0x4121 |

A label column in a sheet with no Y column at all (text-only sheet, X plus label, labels only) is
reported by Origin 2016 as unused (LabTalk type 2), whatever "belongs to" says [orig, checked with six
small files: it holds only when no Y column exists anywhere in the sheet; a text Y column counts, and the
Y column may lie left or right of the label]. The writer therefore writes such a column as unused and
lists a warning, so the file says what Origin will show.

The high byte of the type word is 0x5 for X, 0x6 for Y and 0x4 for an unused column in files from
Origin, and the low byte is 0x30 for error columns and 0x20 for Z and label columns [obs]. The
column record also marks error columns in byte `0x20`: 3 for a Y error and 0x13 for an X error [obs].

### 13.4 Windows, layers, column records

Window header (195 bytes plus the label and one NUL, so a window without long name has 196) [orig]:
the short name at `0x02` (25 bytes), the window rectangle at `0x1B` (left, top, right, bottom as 16-bit numbers)
followed by the screen size 1920 x 1080 [obs], at `0x29` the number of the active sheet [inf], the
text `ORIGIN` at `0x45`, two time stamps at `0x73` and `0x7B` (creation and change, Julian day
numbers as doubles, for example 2461000.5), the label at `0xC3`. The writer puts each new window 26
pixels right of and below the one before (as Origin does) and writes the long name as the label,
without the storage tail Origin adds (section 4.1). The other non-zero bytes of the header are
constants copied from Origin [obs] and were not examined.

Layer header (361 bytes) [orig]: byte `0x02` is 0x5E (bit 0 clear: a workbook sheet), the name at `0xD2`
(31 bytes at most) after the two bytes `Pd`, at `0x17` a double with the number of columns (at most
4), at `0x2B` and `0x151` the number of columns as a 16-bit number, at `0x101` a 16-bit counter that
runs through the sheets of the project from 802 [obs], at `0x12D` the position of the sheet in its
workbook [obs]. Many other constant bytes were copied from Origin [obs].

Column record (515 bytes) [orig]: `0x00` is 0x10; `0x04` the data-set number (16 bits, so a project
holds at most 65535 columns); `0x07` and `0x0B` the same first and after-last positions as in the
data-set header [obs]; `0x11` the role; `0x12` the short name (12 bytes, so 11 characters; the name
in the data set is the full one); `0x1E` is 9; `0x23` "belongs to"; `0x25` the type word; `0x33`
is 1; `0x39` is 0x20; `0x4A` is 0x43 for the first two columns of a sheet and 0 for the others
(Origin does the same); doubles 1.0 at `0x1D2` and `0x1DA`; `0x1E9` is 8.

Note block of a column [orig]: long name, CR LF, units, CR LF, comments, a NUL; trailing empty
parts are left out and a column with no labels gets an empty block. A line break inside a comment is
written as CR LF and read back as CR LF. Long names and units cannot hold line breaks (they are
replaced by spaces). Text that contains `@${` would be read as the start of stored data and is refused.

There are no annotations in a written sheet. Origin writes four per sheet (internal storage objects
named `__LayerInfoStorage`, `__LayerGridStyle`, `__WIPR`, `__WIOTN`); a file without them opened in
Origin with all names, labels and cells intact [orig]. Whether anything is lost without them (for
example the sheet's grid style) was not examined.

### 13.5 The folder tree [orig]

After the notes comes a tree of folders, written here as one root folder that holds every
window. The root folder is named after the project (the file name without extension). Every token
below is a block (section 2.2) unless it says "line", which is a 4-byte number and a newline with no body:

```
block of 4 bytes     S = (total size of everything below up to and including the closing block) - 4
block of 16 bytes    DE 36 10 03, then zeros
block of 32 bytes    byte 2 is 1; at offset 16 and 24 two doubles: the Julian day of creation and of change
empty block
block                the project name, NUL terminated
line                 the number 2
block of 36 bytes    47 11 11 11 01, then zeros
block of 28 bytes    4D 11 11 11 01, then zeros
block of 4 bytes     the number of windows listed (every written window, in file order)
per window:  empty block, block of 8 bytes (u32 0, u32 index of the window), empty block
block of 4 bytes     the number of sub-folders: 0
empty block
block of 8 bytes     00 10 00 00 08 00 00 00        (closes the tree)
```

The results log note before the tree is: the empty block, a block of 80 zero bytes, the block
`ResultsLog` + NUL, a block of one NUL, an empty block. Origin 2016 writes the log window in every project; the writer
writes it empty.
Folders with sub-folders (several `Folder` blocks nested inside the root) are written by Origin with
the same pieces repeated; this writer does not write them. In a project with folders, windows are
listed with the index of their position in the file; the 53 windows of one folder of a private
project were numbered 0 to 52 [obs].

### 13.6 The attribute records [orig]

The file ends with eight records. Each has a 28-byte head, a newline, a body and a newline (the body
and its newline are as in section 2.2; an empty body is just the newline). The head is the number
0x1000, the index 0 to 7, the size of the body and 16 zero bytes. The bodies written:
0 and 1 empty (Origin keeps the command history and the variables of the session here, as text
that begins with `<OriginStorage>`; the writer leaves them empty), 2 and 3 the 20 bytes `25 11 11 11 00 00 01 00 ...` and
`28 11 11 11 00 00 01 00 ...` with zeros, 4 empty, 5 the 24 bytes `3E 11 11 11 01` and zeros, 6 four
zero bytes, 7 the 32 bytes `5D 11 11 11 01 00 01 00` and zeros. Records 5, 6, 7 were identical in all
files from Origin that were examined; records 0 to 3 are larger in files from long sessions.

### 13.7 What Origin needs [orig]

Tried in Origin 2016 with small files; "opens" means `Origin.Application.Load` returned true and the
workbook list was complete.

| file | result |
|---|---|
| a project from Origin without annotations in the sheets | opens, all names and cells intact |
| the same, cut after the window list except for the line `\0` (nothing after the windows but the parameter terminator) | Origin stops responding (the COM call fails with "remote procedure call failed") |
| the same file cut at the end of the notes; cut after the project name of the tree; cut inside the first attribute record; and a file with no tail at all (with and without annotations) | the same failure, each time |
| written by this writer, attribute records 0 and 1 empty, no annotations, no label storage | opens, everything intact |

So the whole tail through the last attribute record has to be present; the annotations, the storage
text in labels and the session history are not needed. (The files with a cut tail and the one without annotations were made by another
session by cutting or editing a small project that Origin wrote; they were opened here again to check.)

### 13.8 Verification

`cargo test -p sheaf-io` writes projects and reads them back with the reader of this crate: names, long
names, units, comments, roles, text, numbers bit for bit, missing values, 20000 rows, accents,
several sheets. `crates/io/tests/fixtures/sheaf-written.opj` is such a file (written by this writer, never
by Origin; a test checks that the writer still makes exactly these bytes).

Origin itself: `tools/origin-ref/opj-verify-written.ps1` loads each file in a hidden Origin 2016 through
COM (with a time limit; modal dialogs would otherwise hang a hidden instance), lists the workbooks,
sheets, short names, long names, units, comments, roles and cells, and compares with the listing of
our reader (`opj-compare-cells.py`, numbers bit for bit). The files come from
`cargo run -p sheaf-io --example opj_write_samples -- FOLDER`.

Files opened in Origin 2016 (hidden instance, one run per file, `Load` returned true each time):

| file | content | workbooks / sheets / columns | cells compared | result |
|---|---|---|---|---|
| `sheaf-written.opj` (the fixture) | all roles, text, missing values, accents, three-line comment, a gap at the top of a column | 2 / 3 / 10 | 33 (every row) | no difference |
| `kinds.opj` | the same plus a third sheet in the first workbook | 2 / 4 / 14 | 48 (every row) | no difference |
| `limits.opj` | 100-character long name, 300-character comment, 31-character sheet name, 24-byte data-set name, 252-byte text, a column name with a space, accented window, sheet and column names, a window without long name, extreme doubles | 3 / 3 / 6 | 17 (every row) | no difference, except that Origin's listing shows -0.0 as 0 (the listing script prints through .NET, so whether Origin kept the sign was not established) |
| `wide.opj` | 200 rows, 13 columns per workbook, gaps, text with accents and degree signs | 3 / 3 / 39 | 7797 (every row) | no difference, numbers to the last bit |
| `edge.opj` | a sheet with no columns | 2 / 3 / 2 | 3 | no difference |
| `empty.opj` | no window at all | 0 / 0 / 0 | 0 | opens, no workbook |
| `big.opj` | 3 columns of 100000 rows | 1 / 1 / 3 | 72 (sampled: first and last rows and 20 spread rows) | no difference |

"No difference" means window names, long names, sheet names, short names, units, comments, roles,
used row counts and every listed cell agree between Origin and our reader, and the reader's result
equals the project that was written (the cargo tests). Not checked: how the window looks in Origin
(column widths, window position; no picture was taken), saving the opened file again from Origin, other
Origin versions (only 2016 was available; the file claims version 9.3 and Origin 7 to 9.1 were not
tried), and everything that is neither a workbook nor a graph.

### 13.9 Graph windows

Code: `write/graph.rs` (model to blocks), `write/glayout.rs` (byte positions), `write/graph_tables.rs`
(constant bytes), `write/graph_tests.rs`. Tags as above.

**Order [orig]** (a graph is listed among the windows in project order, next to workbooks; its curves
point to data sets by position, so every column of the project is numbered first and a graph may come
before its workbook):

```
window header (195 + label)          page size at 0x3D / 0x3F, rectangle at 0x1B
per layer:
  layer header (361)                 frame, axis limits, steps, scale kinds, minor ticks
  annotation blocks                  per object four blocks: header (131), style (102), text, empty
                                     objects in the order: X title (XB), Y title (YL), free text (Text, Text1, ..), Legend
  an empty block                     (end of the annotations)
  curve records                      per curve a 515-byte record and an empty block; a Y error plot
                                     directly after the curve it belongs to
  two empty blocks                   (end of the curves, the extra block)
  axis blocks                        group X: six 515-byte blocks and an empty block; group Y the same;
                                     group Z: six fixed blocks and an empty block
an empty block                       (end of the layer list)
```

**Layer header [orig].** Frame at `0x71` (left, top, right, bottom, 16 bits, 1/600 inch of the page),
the rectangle around the frame that holds tick labels and titles at `0x79`. Per axis (X at `0x0F`, Y at
`0x3A`): from, to and step as doubles, then the number of major ticks, a flag for "limits follow the
data", the page units per axis unit (negative for Y) and at `0x37` / `0x62` the minor tick count and the
scale kind (0 linear, 1 log10). The rectangle at `0x79` is written as an estimate and Origin draws the same
picture [inf].

**Axis blocks [orig].** The six blocks of a group are: minor grid, major grid, tick labels, axis line and
ticks, title, opposite axis. Tick label size in points at `0x15` of block 2; "shown" is bit 6 of byte
`0x26` in every block; the axis line width is at `0x15` of block 3 in 1/500 point and byte `0x25` of that
block holds the major tick direction in bits 4 and 5 and the minor one in bits 6 and 7 (0 none, 1 in, 2 out,
3 both). Colours are four bytes: the position in Origin's colour list, then the same position or the
three bytes red, green, blue and 1 for a free colour. A text column as X sets the flag at `0x23` and
`0x25` in blocks 2 and 4 of the X group; Origin then prints the texts of the column under the bars.

**Curve records [orig].** Type words: 200 line, 201 scatter, 202 line and symbol, 203 column, 487 Y error
(a 515-byte record whose `0x04` is the data set of the error column and `0x23` the Y data set of the curve).
Data-set numbers at `0x04` (Y) and `0x23` (X, 0 for row numbers), first and after-last used row at `0x07`
and `0x0B`. Line colour at `0x0F` and `0x16A`, dash at `0x12`, connection at `0x11` (1 straight, 5 spline,
11 and 12 steps), width at `0x15` in 1/500 point, symbol shape at `0x17`, open flag at `0x18`, size at
`0x19` in 1/500 point, symbol colour at `0x132`, column fill colour at `0xC2`. Byte `0x06` is 9 so
that no curve is in a plot group (a group gives every curve the colour of the first one [obs]).
Origin counts the objects of a project with a running number that is kept at `0x1E2`.

**Annotations [orig].** The text is Windows-1252 with Origin's escapes (bold, italic, super and
subscript, `\l(n)` for a legend icon, `%(n)` for the name of curve n, `%(?X,@LG)` / `%(?Y,@LG)` for an
axis title made from long name and units). The shorter `%(?X)` gave the long name only in Origin 2016;
`@LG` gives "Name (units)" as Sheaf does [orig]. The rectangle of an object is in page
units, the text size in points at `0x04` of the style block, the angle in tenths of a degree. Origin seems
to place axis titles itself: two files that differed only in the written position of a Y title gave
identical pictures [obs]. A legend without a frame has its frame style and background colour set to "none".

**Written, and not written.** Written: page size; layers with frame; X and Y axes with linear or log
scale, limits (a round range is chosen when the model leaves them open), major step, minor tick count,
tick directions, tick label size and visibility, line width, colour, opposite axis, grids on or off;
titles; line, scatter, line and symbol and column plots with colour, width, dash, connection, symbol
shape, size and open or filled; Y error bars; free text with size and rotation; the legend with its
place, frame and text; categories from a text X column. Named in a warning, not written: page background
colour, fonts other than Origin's default, text colours outside Origin's list (the nearest listed colour
is used), grid colour, width and dash (Origin's default grid style appears), legend background colour, a
plot's row range, symbols on every n-th point only, hidden plots, plots whose data are not columns of a
workbook, axis ranges that follow the data in a way other than Origin's own (the limits for the current
data are written), and graphs with no layer (not written at all).

**Minimum records [obs, partial].** Every block above is written because Origin writes it. The constant
bytes come from graphs that Origin 2016 saved (`opj-make-from-labtalk.ps1`; the tables in
`graph_tables.rs` are the non-zero bytes of each block with the fields we set cleared). Compared with
Origin's own files, the writer leaves out the stored layer information, plot groups and the storage text
of the project, and Origin still opens the files. A systematic removal of single blocks to find the
smallest set that opens was not done for graphs; the whole set above is kept.

**Frame and extreme ranges [obs].** A layer frame with no width, a negative corner or a size beyond the page
loads in Origin 2016 but exports a blank picture, so the writer replaces it by a default frame (0.15, 0.12,
0.78, 0.76 of the page) and warns. A log axis over data from 1e-300 to 1e300 came out in Origin with a limit
range (about 1e269 to 1e281 drawn) that does not cover the data; the writer gives the limits it computes and
does not warn. The cause (writer or Origin's own limit) was not traced, and such data is outside the
range any real project holds.

**Verification.** Round trip: `cargo test -p sheaf-io` writes graphs, reads them with this crate's
reader and compares the models (all plot kinds, shapes, dashes, connections, log axes, automatic ranges,
error bars, text X, two layers, warnings; damaged copies of written files never make the reader panic).
The reader does not read grid lines, legend text or fonts (section 10.7), so those were checked in
Origin only. Origin 2016 (hidden instance, `opj-verify-graphs.ps1`) returned true from `Load` for each of
the 8 files of `cargo run -p sheaf-io --example opj_write_graphs -- FOLDER`, listed all 18 graph windows,
and exported each as a PNG. The report Origin printed for `g-basic` (limits, steps, scale kinds, frame)
agrees with what was written; the reports of the other files were not compared line by line. The pictures
were compared by eye with `sheaf export` of the same file (the pictures are in the scratch folder only).

Matches: page and frame places, axis limits, steps, minor ticks and tick directions, log axes (Origin
prints 1, 10, 100 where Sheaf prints powers of ten), line colours, widths, dashes, straight and step
connections, all symbol shapes filled or open, column bars with error bars and free text labels, two layers
on one page, the owner-style cyclic voltammogram overlay (three curves, box, titles), the current against
time curve and the column chart with error bars and text labels (a to d).
Differences seen: (1) Sheaf's picture of a read-back file numbers the categories 1 to 6 where Origin prints
the station names from the text column, because the reader does not read a text X column. (2) Origin draws
a legend icon for straight lines, symbols and columns but not for spline curves; the same graph with
straight lines had icons [obs]; the cause was not found. (3) Origin writes tick labels as plain decimals
(0.000025) where Sheaf uses 2.5x10^-5; when the frame leaves too little room, Origin cuts the Y title off at
the page edge (seen in the current against time graph), and the model's tick number format (decimal, scientific) is not written, so the writer warns and Origin's decimal labels stay.
(4) Origin's column bars are wider than Sheaf's. (5) Sheaf's picture of a read-back file has no grid lines,
since the reader skips them; Origin shows them in its default style. (6) Origin's legend of a column plot has
a filled box icon and a frame; the reader keeps neither.
Not checked: Origin versions other than 2016; opening a written file in the visible Origin window and saving
it again; other machines or fonts.


# Origin project file format (`.opj`)

Status: written while building `crates/io/src/opj/`. Covers what the reader needs: workbooks,
sheets, columns and their values (checked cell by cell against Origin, section 12) and basic graphs
(section 10). Matrices and notes are only sketched.

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
| `0x0F` | u8 | line colour as a position in the colour list (10.6); 0 when automatic |
| `0x11` | u8 | line connection: 0 none, 1 and above a line (values 1 to 8 all drew straight lines through 8 points; the other kinds were not told apart) |
| `0x12` | u8 | line style: 0 solid, 1 dash, 2 dot, 3 dash dot, 4 dash dot dot, 5 short dash, 6 short dot, 7 short dash dot (the `set -d` values, modulo 8) |
| `0x15` | u16 | line width in 1/500 point (`set -w 1000` is 2 points) |
| `0x17` | u8 | symbol shape: 0 none, 1 square (the default); the others are guessed: 2 circle, 3 up triangle, 4 down triangle, 5 diamond |
| `0x19` | u16 | symbol size in 1/500 point (`set -z 12` is 6000) |
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
their columns, type, colours, line width and style, symbol shape and size; error columns, which are
attached to the plot they belong to. A Y error plot (487) is attached; other error kinds are not.

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

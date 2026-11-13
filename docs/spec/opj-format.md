# Origin project file format (`.opj`)

Status: written while building `crates/io/src/opj/`. Covers what the reader needs: workbooks,
sheets, columns and their values (checked cell by cell against Origin, section 12) and basic graphs
(section 10). Matrices and notes are only sketched. Section 13 describes what the writer does and
what Origin needed to open a written file.

Provenance tags used on statements:

- **[obs]** seen directly in project files (the five projects written by Origin /
  version 9.3, and the 50 sample projects that ship with Origin, which were saved by
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

Every column of an Origin file is stored as a 10-byte-or-wider "text or number" cell,
whether or not it holds text; older versions store plain 8-byte doubles [obs]. "No value" in a
double is the number -1.23456789e-300 [obs]. Text is Windows-1252 (an ANSI code page) [inf].

## 2. Container layout

### 2.1 Signature line [obs]

`CPYA <major>.<build> <n> [W32|W64] #` followed by a newline. Examples seen: `CPYA 4.3168 226 W64 #`
(Origin), `CPYA 4.2889 890#` (version 8.1, no platform field), `CPYA 4.3165 195 W64 #`
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

- Bit `0x0100` set: **text-or-number cells** (section 3.4). Seen in every column of Origin
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

Other label rows (parameters, user-defined rows) are also in this note storage and in the
annotations; they are not decoded [unknown]. The "set column values" formula is not in the note: it is
a sheet annotation (section 13.10).

### 4.5 Sheets and sheet names [obs]

Each layer of a workbook window is a sheet, in order. Sheet names: layer header offset `0xD2`. The
data sets of a sheet carry a `@<n>` suffix for every sheet but, usually, the first; `n` is not the
sheet index (section 3.1), so the link must be made through the column records. The column order
within a sheet is the order of its column records, not the data-set order [obs].

## 5. Annotations and other layer contents

### 5.1 Annotation [obs]

An annotation is a header block (123 or 131 bytes), then block 1, block 2 and block 3. The reader
steps over them (the reader keeps the small ones). In a workbook sheet the annotations hold the sheet's
embedded storage and the "set column values" formulas (one annotation per formula, section 13.10); in a graph they hold text labels, lines, legends and colour scales [pub].
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
- **Project tree.** The project name and the folders (names as NUL-terminated strings), each with the positions of its
  windows, and per folder a 32-byte record with two timestamps as doubles; layout in section 13.5 and, for
  nesting, section 13.11 [obs][orig].
- **Attribute records.** Eight records at the very end of the file; record 2 lists the column formulas that
  recalculate (section 13.10).
- **Further blobs.** High-entropy data at the end of the file (embedded storage, compressed). Not understood.

The reader reads the folder tree and record 2 after the window list (`opj/tree.rs`); a file whose tail does not
look like section 13.5 simply has no folders and no recalculation table.

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
| 9.2 - 9.3 (Origin) | 111 | 147 | 515 | worksheet columns are text-or-number cells (cell size >= 10); plain doubles still occur for generated data |

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
9.x (the owner's projects and graphs built in Origin). Projects saved by Origin 7.0 to 8.6 have
the same block structure, and the rule in 10.0 was checked against them, but the offsets of the page
size, frame, axes and curve records were *not* checked on any pre-9 file. For a file older than 9.0
the reader still imports the graphs that 10.0 identifies, and always adds a warning that graphs of
such a file are not verified and may be missing or wrong (also when no graph was read, since a
missed graph would otherwise go unmentioned). Offsets below were found by building graphs in Origin with known settings
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

- Over the 54 sample projects of Origin, the five owner projects and 634 windows in all, the
  bit is clear in every layer of all 360 windows that are workbooks by the name rule of section 4,
  and set in every layer of every other window that has axis blocks except two; those two (one in
  `2D Bin and Fit.opj`, one in `Calling Fortran.OPJ`, both Origin 8.x) have the flag word `0xDE`, and
  Origin lists both as matrices.
- Cross-check against Origin itself, on 33 projects (23 saved by versions 7.0 to 8.6, 10 by
  9.x including the five owner projects): `tools/origin-ref/opj-list-windows.ps1` opens a copy of each
  in a hidden Origin and lists the windows of `doc -e W` (workbooks), `doc -e P` (graphs) and
  `doc -e M` (matrices). Before the fix the reader named 37 graph windows that Origin does not list
  as graphs (workbooks and matrices of 8.x projects, for example `Book1` in `Simple XY.opj`, which
  has one workbook and one graph). After the fix it names none: every graph window the reader finds
  is a window Origin lists as a graph.
- The rule does not change anything for 9.x projects: in all 9.x files seen, every window with axis
  blocks has the bit set (the reader's graph count for the five owner projects is the same before
  and after: 26, 1, 7, 8 and 17).

What the rule does not find, and why (looked into on 2026-11-12: Origin's own list of each
project through `tools/origin-ref/opj-list-windows.ps1`, compared with a block-by-block walk of the
same files; 14 projects with Origin's list, the five owner projects and nine samples, 531 windows):

- **Graphs that are not windows in the file.** Most graphs Origin lists that the reader does not
  read have no window record at all: 453 of the 658 graphs Origin listed in those 14 projects. Their
  names are `sparkline`, `sparkline1`, ..., `FitLine...`, `ResIndp...`, `ResidualA`, `ANOVA3Way...`
  and similar. Origin builds them when it opens the project, from data kept inside workbook sheets
  and analysis reports. In the owner's project "Integration of Delithiation SF data" the file has
  41 windows, 34 workbooks and 7 graphs, and the reader reads all 7. The other 34 graphs Origin lists
  are the sparklines of the six workbooks whose sheets show a sparkline row, one per column (5, 5, 5,
  5, 7 and 7). Such a workbook's first layer annotation (`__LayerInfoStora`) holds a block 2 of
  8 to 12 KB that starts with `@${[0|5|_Storage_Ebdded_pages_Data_|...`; in other workbooks that
  block is a stub of a few bytes. The payload is escaped and probably compressed. It is not plain
  zlib or deflate, and it was not decoded [obs for the location, inf for the content]. A sparkline
  is a small preview of one column inside the sheet, not a figure the owner made, so the reader
  leaves these out on purpose. The other four owner projects have no such gap: the reader's graph
  count equals Origin's (26, 1, 8 and 17).
- **Windows with the bit but no axis blocks** (39 windows in all the files seen): 37 are OpenGL 3D
  graphs (an annotation named `__O3DGL`, flag byte `0x5F`), 1 is a radar plot (`Radar`), and 1
  (`Graph1U` of the sample "2D and Contour Graphs") has no axis blocks in its first layer but has
  them in layers 2 to 4. None is in the owner's projects. A rule that needs only the bit would
  find all of them, and no workbook or matrix: over the 531 windows, the bit is set on exactly the
  205 windows Origin lists as graphs. But the reader has nothing to draw for 3D or radar layers, so
  the rule keeps the axis-block test.

The bit's meaning ("this layer is a graph layer") is inferred from the correlation above; no second
source states it [inf].

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
0 minor grid, 1 major grid, 2 tick labels, 3 axis line and ticks, 4 tick labels on the opposite
side, 5 the opposite axis line. (An earlier version of this section called block 4 the title; the
double-Y study of graphing.md 12.7 showed what it is.)

- Block 2, byte `0x15`: tick label font size in points. Byte `0x26`: bit `0x40` set means the
  labels are shown (cleared by `layer.x.showLabels = 0`).
- Block 3, u16 at `0x15`: axis line width in 1/500 point (1500 is 3 points). Byte `0x25`: bits 4 and
  5 give the major tick direction, bits 6 and 7 the minor one: 0 none, 1 in, 2 out, 3 both (a default
  Origin graph has `0xA0`, both out; the owner's Y axis has `0x50`, both in, which matches his figure).
  The meaning of 1 and 3 was inferred from these two cases, not from a rendering of each.
- Block 3, byte `0x26`: bit `0x40` set means the axis line and ticks are drawn on the primary side
  (bottom for X, left for Y). u16 at `0x0F`: the axis colour; `F7 FF` means "automatic" (the colour
  of the layer's plot), 0 black.
- Block 4, byte `0x26`: bit `0x40` set means tick labels are drawn on the opposite side (top for X,
  right for Y).
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
| `0x06` | u8 | plot group membership: bit `0x20` on the first plot of a group, bit `0x10` on the others; the other bits (`0x01`, `0x02`, `0x08`) vary between files and are not used (13.13; seen in Origin's own files as `0x21` then `0x11` for a group of line plots, `0x20` then `0x10` for two, and `0x0B`, `0x03`, `0x01`, `0x00` outside a group; graphing.md 14.7) |
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
| `0x1B2` | f64 | X offset of the curve in a stacked layer, in axis units; 0 in every file seen, and Origin ignored a value written there (13.12) |
| `0x1CA` | f64 | Y offset of the curve in a stacked layer, in axis units: the offset Origin drew it with; Origin moves the curve by it when the layer header byte `0xFD` is `0x50` (13.12) |
| `0x1C2` | f64 | Y factor of the curve (1.0; 3.0 stretched a curve three times) [obs, 13.12] |

"Automatic" colours follow the list by plot position: the first plot black, the second red, and so
on. The reader gives each automatic plot the list colour of its position in the layer. Note that
`set -c 1 -w 1000` (several options in one call) is silently ignored by Origin; each option
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

### 10.8 Legend frame [obs]

The legend object (name `Legend`) has a 131-byte header and a 102-byte style block (10.4). Three colour slots of
the style block say how it is drawn, each as 4 bytes (a list position then three zero bytes, or `FC FF FF FF` for
"none"): at `0x1A` the background, at `0x2A` the colour of the frame, at `0x32` a third colour that is "none" in
legends and 0 in free text. Origin's own legends in the owner's projects
all have `0x1A` = 17 (white), `0x2A` = 0 (black), `0x32` = none, byte `0x28` of the style block `0xFA`, byte `0x29`
of the header `0x80` and byte `0x28` of the header `0x20`: a legend with a black frame on a white background. Free text objects
have `0x1A` and `0x2A` set to none, `0x32` = 0 and header byte `0x29` = 1: no frame and no background. The writer's
legend without a frame (graph_tables `ANN_LEGEND_*` with the fields cleared as in `glayout.rs`) has the text object's
colours. The reader takes `0x2A` = `FC FF FF FF` as "no frame" and anything else as a frame, which fits every case
seen. The other frame kinds of the Frame tab (shadow, marble and so on) were not produced and their bytes are unknown,
so they read as a plain frame and are written as one. The background colour is not read or written.
A legend in a file with no legend object keeps Sheaf's template default (no frame, graphing.md 6.4).

### 10.9 Plot groups [obs]

Plots made by one plot command are a *group*: their curve records follow one another, the first with bit `0x20` in
byte `0x06` and the others with bit `0x10` (10.5). Nothing else in the records or the layer header marks a group, so
the group is the run that starts at a `0x20` curve and goes on while the following curves carry `0x10`; a curve with
neither bit ends it, and a run of one curve is not a group. Origin gives the members the colours of its colour list
(10.6) in order, black first, whatever the records hold (13.13), which is the group's colour increment "by one"; the
model keeps the group as `Plot::group` (a number, equal for the members of one group in a layer), the reader sets
the members' colours from the list, and the writer writes a run of neighbouring plots with the same group number as
a group again. Increment settings other than the colour (line style, symbol shape, width), "independent" mode and
the Group Members list live in places that were not found (13.13); the line style, width and symbol of every member
are read as stored.

### 10.7 What the reader does with it, and what it leaves out

Read: page size; for each layer the frame, both axis ranges, steps, scale (linear or log10), minor
ticks, tick direction, tick label size and visibility, axis line width, the box; the titles of the
bottom and left axes with their font size; free text (one label per line, centred on the object's
rectangle with an estimated text width); a legend (present or not, and its position); plots with
their columns, type, colours, line width and style, symbol shape and size; the gap between the bars
of a layer's column plots (from its first column plot); error columns, which are attached to the
plot they belong to. A Y error plot (487) is attached; other error kinds are not.

Also read: the offsets of stacked curves, and the second layer of a double-Y graph as a linked layer
(graphing.md 12.7 and 13.10, "Reader rule"). A layer whose curves are moved gets `stack =
Individual` with each curve's stored offsets, so it draws exactly as Origin saved it, whatever mode
Origin used (the mode bytes are not needed for that). The offsets are kept by the Y column of the
curve, and `OpjImport::into_project` renumbers them together with the plots.

Not read (the figure still draws, with the defaults of the template): the top axis title, and the
right one except on a right-axis layer;
the Z axis; other scale types (drawn linear, with a warning); tick label font, bold and colour;
number formats; the colour and style of grids and of
the frame; text colour; line and arrow objects; the legend's background colour and the kind of its frame
(only whether it has one is read, 10.8); the increments of a plot group other than the colour (10.9).

Known differences from Origin:

- Plots in a *group* take their colour from Origin's colour list by their place in the group; the file keeps
  the value that was set on each plot, and the reader replaces it by the list colour (10.9). Width and style
  are read as stored: after `set` on the second plot only, Origin showed the group's value and the reader
  shows the stored one.
- Column plots are drawn as bars from zero, 70 percent of the smallest X spacing wide, filled with the
  fill colour and outlined with the line colour. Origin's handling of bar border and fill colour was not
  decoded: a column plot whose `set -c` colour was changed is black in Origin and cyan outlined here.
- Horizontal bars, stacked bars, area, 3D, contour, polar, pie and box plots are not read.
- Layers that share one frame are linked only when the later one shows right-side tick labels
  (double Y). Other overlapping layers stay independent and draw over each other, as before. A
  right-axis layer that also shows its left axis has only the right side drawn (the import says so).
- The legend frame is read (10.8) as drawn or not; Origin's other frame kinds (shadow, marble, ...) are not told
  apart and draw as a plain line frame.

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
- Hidden columns, column widths, number formats, the user parameter rows of a worksheet, and the
  "before formula script" and the out-of-date mark of a formula column (section 13.10).
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
not sampled in the large project, hidden columns, column widths and number formats, and matrices
(`doc -e M` was not run). Formulas and folders were checked in section 13.10 and 13.11.

## 13. Writing

The writer is `crates/io/src/opj/write/` (`sheaf_io::opj::write`, `write_file`). It writes the
workbooks and graphs of a project in the format of Origin (version 9.3). Graphs are 13.9; column formulas
are 13.10 and the folder tree with sub-folders 13.11; what a graph holds that the file does not is named in a warning. Tags: **[orig]** checked by opening the written file in Origin
(sections 13.7 and 13.8), **[obs]** observed file-format values, **[inf]** a guess that Origin
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
- A column with text and numbers (Text & Numeric, import-ascii.md section 15) uses the same cell
  size for both kinds: numbers are `00 00`, the double and NUL padding up to `size`. The writer and
  the reader keep both kinds (a column of only text cells reads as a text column).
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
reported by Origin as unused (LabTalk type 2), whatever "belongs to" says [orig, checked with six
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
without the storage tail Origin adds (section 4.1). The other non-zero header bytes are fixed format values [obs]; their individual meanings
have not yet been identified.

Layer header (361 bytes) [orig]: byte `0x02` is 0x5E (bit 0 clear: a workbook sheet), the name at `0xD2`
(31 bytes at most) after the two bytes `Pd`, at `0x17` a double with the number of columns (at most
4), at `0x2B` and `0x151` the number of columns as a 16-bit number, at `0x101` a 16-bit counter that
runs through the sheets of the project from 802 [obs], at `0x12D` the position of the sheet in its
workbook [obs]. The remaining fixed byte values have not yet been assigned individual meanings [obs].

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
`ResultsLog` + NUL, a block of one NUL, an empty block. Origin writes the log window in every project; the writer
writes it empty.
Folders with sub-folders are written by Origin with the same pieces repeated inside the root, and this
writer writes them too (section 13.11). In a project with folders, windows are listed with the index of
their position in the file; the 53 windows of one folder of a private project were numbered 0 to 52 [obs].

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

Tried in Origin with small files; "opens" means `Origin.Application.Load` returned true and the
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

Origin itself: `tools/origin-ref/opj-verify-written.ps1` loads each file in a hidden Origin through
COM (with a time limit; modal dialogs would otherwise hang a hidden instance), lists the workbooks,
sheets, short names, long names, units, comments, roles and cells, and compares with the listing of
our reader (`opj-compare-cells.py`, numbers bit for bit). The files come from
`cargo run -p sheaf-io --example opj_write_samples -- FOLDER`.

Files opened in Origin (hidden instance, one run per file, `Load` returned true each time):

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
axis title made from long name and units). The shorter `%(?X)` gave the long name only in Origin;
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
data are written), and graphs with no layer (not written at all). Section 13.12 adds what was written later:
stack offsets, the right Y layer of a double-Y graph with its legend, the gap of column bars, axis sides and
automatic axis colours; the warnings of this paragraph for those no longer apply.

**Minimum records [obs, partial].** Every block above is written because Origin writes it. The constant
bytes come from graphs that Origin saved (`opj-make-from-labtalk.ps1`; the tables in
`graph_tables.rs` are the non-zero bytes of each block with the fields we set cleared). Compared with
Origin's own files, the writer leaves out the stored layer information, plot groups and the storage text
of the project, and Origin still opens the files. A systematic removal of single blocks to find the
smallest set that opens was not done for graphs; the whole set above is kept.

**Frame and extreme ranges [obs].** A layer frame with no width, a negative corner or a size beyond the page
loads in Origin but exports a blank picture, so the writer replaces it by a default frame (0.15, 0.12,
0.78, 0.76 of the page) and warns. A log axis over data from 1e-300 to 1e300 came out in Origin with a limit
range (about 1e269 to 1e281 drawn) that does not cover the data; the writer gives the limits it computes and
does not warn. The cause (writer or Origin's own limit) was not traced, and such data is outside the
range any real project holds.

**Verification.** Round trip: `cargo test -p sheaf-io` writes graphs, reads them with this crate's
reader and compares the models (all plot kinds, shapes, dashes, connections, log axes, automatic ranges,
error bars, text X, two layers, warnings; damaged copies of written files never make the reader panic).
The reader does not read grid lines, legend text or fonts (section 10.7), so those were checked in
Origin only. Origin (hidden instance, `opj-verify-graphs.ps1`) returned true from `Load` for each of
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

### 13.10 Column formulas [orig]

A column that was filled with "Set Column Values" keeps its formula. The pieces below were found by making small
projects in Origin with known formulas (`csetvalue col(B) formula:="col(A)*2" recalculate:=1`, the same with
0 and 2, formulas of 8 to 100 characters, one with two input columns, one that fills rows 2 to 4, one with a
"before formula" script, several in one sheet, in a second sheet and in a second workbook) and comparing their
bytes with the same project without the formula (`tools/origin-ref/opj-make-from-labtalk.ps1`,
`opj-dump-blocks.py`). Code: `write/tail.rs` (bytes), `write/formulas.rs` (which columns a formula reads),
`opj/tree.rs` and `Builder::apply_formula` in `opj/mod.rs` (reader).

**The text: one annotation per formula [orig].** The formula is an annotation of the sheet (section 5.1), placed
after the sheet's other annotations, if any, and before the empty block that ends the list. When a sheet has
several formulas their annotations follow each other with the last column first (the order of creation reversed
in the files made one formula at a time). The annotation has the usual four blocks:

```
header, 131 bytes   0x02 = 06 (the kind)
                    0x13 two doubles, constant: 85 EB 51 B8 1E 85 BB BF  92 24 49 92 24 49 C2 BF
                    0x23 u32  size of block 1
                    0x28 constant: 10 01 3E 00 3E 00 C2 FF C2 FF
                    0x46 the short name of the column, NUL terminated (28 bytes)
                    0x71 = 08
block 1             the text, NUL terminated, zero padded to at least 20 bytes (size = max(20, length + 1))
block 2, block 3    empty
```

The text is the formula as typed (`col(A)*2`), Windows-1252. For a formula that fills only some rows the text
continues with `;` CR LF `#$[first:last]`, the rows 1-based and inclusive (`col(A)*2;\r\n#$[2:4]`; Origin's own
`wks.col.formula$` returns the part before the semicolon). A "before formula" script is stored in the same place
as `;` CR LF `#` and the script (`col(A)*2;\r\n#col(C)=7`). The reader splits the rows off and keeps a script as part of
the formula text (with a warning). Row ranges are kept by Sheaf only for columns that recalculate (a formula that ran
once has no range in the project model), so the reader drops the range of a column with mode None.

**The recalculate mode: not in the annotation [orig].** Annotations of a formula with mode None, Auto and Manual
are byte for byte the same. The mode lives in two other places:

- the column record has a number at `0x1E2` (16 bits; zero for a column that takes no part). The columns a
  recalculating formula reads and the column it fills have one; all other columns have 0;
- attribute record 2 (section 13.6) lists the recalculating formulas. In a file without any it is the 20 bytes
  `25 11 11 11 00 00 01 00`, zero count, zeros. With formulas: the same 8 bytes, the number of entries (u32), a zero
  u32, then the entries one after another, then a zero u32. An entry is 284 bytes for a formula that reads one
  column and 4 more for each further column. Its parts, in order: tags `24 11 11 11 22 11 11 11 00 00 01 00`;
  the size of the entry minus 68 (u32); `23 11 11 11 06 00 01 00`; the **input list**: its size in bytes
  (4 per number), the column numbers, a zero u32; the **output list** in the same form with the number of the
  column that the formula fills; the **mode** (u32: 2 for Auto, 1 for Manual), a number for the formula itself
  (u32, any number nobody else uses) and a zero u32; then `05 00 00 00`, the size of what follows up to a
  fixed point (`0x60` when the creator name has six letters, as the name of the Windows user in the files examined), `2D 11 11 11 01 00 01 00`,
  the time as a Julian day number (double), the creator name (u32 length with terminator, name), and 189 bytes
  of constants that were identical in every entry (they include a 10-byte cell holding the "missing value"
  number -1.23456789e-300 and the words `UT` and `Unknown`; their meaning is not known). The writer uses
  its own name, "Sheaf", for the creator (one byte shorter, so the two size fields are one less). The
  inputs are listed in order of first use in the formula.

A formula with mode Auto or Manual and no input column (`5`, `i`) reads back from Origin as mode None, and the
writer stores such formulas that way (with a warning). So does a formula that reads its own column (Origin's
documented behaviour; Sheaf's model makes it mode None too).

**Two things that are needed for Origin to use the table [orig].**

1. *The numbers must stay out of the range Origin hands out while it loads.* With numbers starting at 0x0323
   (what Origin itself writes in a new session) a file with 12 workbooks of one Auto formula each lost the first 4
   recalculations, 6 workbooks the first 2, 3 workbooks the first one, and a file with 2 workbooks of
   formulas lost 3 of 6: `col(A)` was changed from LabTalk, `run -p au` run, and the formula column did not change,
   although Origin reported the right mode and formula text. The count lost grew with the number of
   windows, so the numbers seem to collide with the numbers Origin gives to the objects it creates while loading.
   The writer starts at 0x4000 and gives each formula, each input column and each output column a new number
   (a column that is both is numbered once); with that every recalculation worked, up to the largest case tried (12
   workbooks, and 6 formulas in 2 sheets and 2 workbooks).
2. *Attribute record 0 must not be empty.* With record 0 empty (the 0-byte body the writer used until now)
   Origin loaded the formulas and their modes, and still did not recalculate; with any
   non-empty body it did (`x` worked; so does the shortest valid storage text `<OriginStorage></OriginStorage>`, which
   the writer writes when a project has recalculating formulas, and only then). Origin keeps the
   command history there (section 13.6).

The 189 constant bytes of an entry are written exactly as Origin writes them; whether any of them could be left
out or changed was not tested.

**Not stored.** The "before formula" script of the dialog (it is stored by Origin, and the reader keeps it inside the
formula text, but Sheaf has nowhere to put it); the state of the lock (green, yellow, red: a Manual formula whose
inputs changed is out of date in Sheaf; in the file nothing of that was found, so the writer warns and the column reads
back as up to date); a mode other than None for a formula with no input columns, with an unparsable text or one
that reads a missing column (stored as None, with a warning); formulas longer than 60000 bytes, with a NUL or
containing `@${` or `;` CR LF `#` (not stored, with a warning).

**Verified.** With `tools/origin-ref/opj-verify-features.ps1` (loads the file in a hidden Origin and prints, per
column, `wks.colN.svrm` and `wks.colN.formula$`, and the folder tree): `cargo run -p sheaf-io --example
opj_write_features -- FOLDER` writes `formulas.opj` (Auto, Manual, None, a formula with two inputs, one with rows 2 to
4, one reading a formula column, a second sheet, a second workbook). Origin reported for each column the same
mode (1 Auto, 2 Manual, 0 None) and the same formula text. Function: after loading, `col(A)` was changed and
`run -p au` was run; the Auto and the Manual formula columns changed (the row range kept to rows 2 to 4, the
None column did not). Origin then saved the loaded file again and the reader of this crate read the same formulas, modes and rows back
from Origin's file. `opj-verify-written.ps1` found no difference in the cells of these files. The cargo tests
(`write/feature_tests.rs`) write and read back every mode, the rows, several sheets and workbooks, the layout
of the table and truncations of the file.

### 13.11 Folders [orig]

The tree of section 13.5 repeats its folder piece for every folder, inside the piece of the folder that holds it. One
folder, in the order of the file:

```
block of 32 bytes    zeros, but byte 2 is 1 for the folder that was open when the project was saved;
                     at 16 and 24 two doubles: Julian day of creation and of last change
empty block
block                the folder name, NUL terminated
line                 the number 2
block of 36 bytes    47 11 11 11 01, then zeros
block of 28 bytes    4D 11 11 11 01, then zeros
block of 4 bytes     the number of windows in this folder
per window:          empty block, block of 8 bytes (u32 0, u32 position in the window list), empty block
block of 4 bytes     the number of sub-folders
the sub-folders, one after the other, each in this same form
```

The root folder has in front of it the 16-byte marker block (`DE 36 10 03`) and, after its last sub-folder, the empty
block and the 8-byte closing block of section 13.5. The size block in front of the whole tree counts everything
after itself except 4 bytes. Found in files made by Origin with `pe_mkdir folder:=F1 cd:=1` (a new window goes
to the active folder): a three-level chain F1 / F1a / F1b, a sibling F2 with two windows, an empty F3, and another
project with windows created in the root after others were made in sub-folders.
Byte 2 of the 32-byte block is 1 for the folder that was active when the project was saved and 0 for the others (the
writer writes 1 for the root). In the private projects the active folder was a sub-folder and its second fixed block
is longer (95 to 99 bytes: `4D 11 11 11 01`, then a storage text that names the last used folder, and zeros); the
reader accepts any length of 5 bytes or more, and the writer writes 28.

Window positions index the window list **in the order the windows are written in the file**. That order is not the
order of the folders and not always the order of creation: in the project with a chain of folders, the first window
(in the root) was written last, after the windows of the sub-folders, and the root listed that last position. It looks like
the order in which the windows were last active [inf]; the position is all a reader needs. The writer writes the windows in the order of the project and lists the windows of each folder by
ascending position (Origin does the same). A window that no folder lists, and one the root lists, are in the root; a window
listed by several folders stays in the first one in depth-first order.

Names: the folder name is a plain NUL-terminated string. `pe_mkdir` accepted 100 characters, spaces, a colon,
a question mark and a backslash; it made a sub-folder of `a/b`, and turned `LOWER` into `LOWER2` next to `Lower` (names are unique
ignoring case within a folder). The writer therefore refuses (like other names Origin cannot store) a name that is
empty, longer than 100 bytes, outside Windows-1252, with a control character, a slash or a backslash, and two
siblings whose names differ only in case, and nesting deeper than 32 levels. A name may repeat in different places.

**Verified.** `opj_write_features` writes `folders.opj` (Book1 in the root, Book2 in F1, Book3 in F1/F1a, Book4 in
F1/F1a/F1b, Book5 and Book6 in F2, an empty F3; the project that was also made in Origin, and the tail of
the two files, taken block by block, is the same except for names, sizes and times) and `deep.opj` (twelve levels,
a name of 45 characters, a folder with a name of 100 characters). `opj-verify-features.ps1` loads each in Origin through COM and walks `RootFolder`, `Folders` and `PageBases`: the folders, their nesting, and the window in
each folder were all as written (the deepest folder had the path of 12 names). Origin saved `folders.opj` again and the
reader of this crate read the same tree back from Origin's file. The four private projects with folders (one
folder with 2, 11, 27 and 53 windows) read as a root with one folder whose windows have the right names. The cargo tests
(`write/feature_tests.rs`) write and read back nested folders, empty folders, graphs in folders, refused names, the depth
limit, the size block and truncations of the file.

### 13.12 Graph features that the file holds: stack offsets, double-Y, bar gap, axis sides and colours [orig]

Code: `write/graph.rs` (`Role`, `LayerCtx`, `unwritten_layer_features`, `axis_spec`), `write/glayout.rs` (`LinkSpec`,
`link_annotation`, `axis_blocks`), tests in `write/graph_feature_tests.rs`. Found on 2026-11-12 by patching single bytes
of files the writer made and loading them in Origin (`tools/origin-ref/opj-verify-graph-features.ps1` prints, per
layer, `layer.link`, `x.link`, `y.link`, `unit`, `showAxes`, `showLabels`, the axis colours, and exports a PNG;
`cargo run -p sheaf-io --example opj_write_graph_features -- FOLDER` writes the test files). Pictures and reports are
in the git-ignored `re-workspace/scratch/opj-write-graphs/`.

**Stacked curves.**
- A layer header byte at `0xFD` set to `0x50` makes Origin move every curve by the Y offset kept in its record
  (f64 at `0x1CA`). It needs no plot group and the curves keep the colours written in their records. With `0x10`
  or `0x40` alone nothing moves. This is how all three modes are written: the offsets are worked out as the model draws
  them (`sheaf_core::stack::places`, shared with the renderer) and written as they are, so Auto, Constant and Individual
  layers all look the same in Origin. A written layer reads back as `Stack::Individual` with those offsets.
- Origin itself writes the Auto mode another way: the curves are a plot group (byte `0x06` is `0x29` on the first
  curve and `0x19` on the others), the layer header f64 at `0x135` holds Origin's "no value" double and the f64 at
  `0x13D` the gap as a fraction (0.1 for 10 %). Origin then works the offsets out again when it loads the file and
  ignores the stored ones. The same fields give the Constant mode when `0x135` holds the step: member k is moved by k
  times the step (steps 1 to 5 and 40 tried). The writer does not use this, because a plot group also hands out the
  group's colours. The offsets Origin computed for one test (60, 170.0489, 176.0959) equal what `places` gives to
  the last digit that was compared, so the rule of graphing.md 13.4 is the one Origin uses.
- The byte `0x13A` of the curve record (any value tried from `0x01` to `0x80`, on the first or on all curves), `0x148`
  and the bytes `0x13`, `0x1B`, `0x1E`, `0x72` and `0x194` changed nothing.
- X offsets: no field was found that Origin honours. The f64 at `0x1B2` was written as 3.0 on one curve, with the `0xFD`
  byte set and with several other values of the bytes `0xFC` and `0xFD`, and the curve did not move. The f64 at `0x1C2` is
  a Y factor (3.0 stretched the curve three times); `0x1BA`, `0x1D2`, `0x1DA` and `0x1AA` changed nothing. The writer
  drops X offsets and says so in a warning. The Y range of the layer header covers the moved curves.

**Double-Y (a layer that Origin links to its parent).**
- Layer header byte `0x0AB` holds the 1-based number of the parent layer. With only that byte set, Origin reports
  `layer.link` = the parent. In Origin's own files `0x12D` is the position of the layer among the layers (1 for the
  second), `0x02E` is 9 and bit `0x80` of `0x0FC` is set; the writer sets them too (they did not change the link).
  `layer.unit` reports 1 (percent of the page) for the written layer, where Origin's own file says 7 (percent of
  the linked layer); where the unit is kept was not found. The layer has the same frame as its parent in the page
  units at `0x71`, so both pictures stand over each other.
- The straight X link is the object pair `X2`, `X1` in the linked layer: a 131-byte header and a 20-byte style block
  whose first byte is `*` (`glayout::link_annotation`). With them `layer.x.link` is 1; without, 0. (With a custom link
  Origin writes the text `X1` in that block instead.) The X range is written the same in both layers.
- Axis visibility, as in graphing.md 12.7: the right layer shows in its Y group the second edge's tick labels (block 4)
  and line (block 5) and nothing on the first edge; its X group shows the top line (block 5) only. Origin reported
  `showAxes` 2 and 2, `showLabels` 0 and 2 for the layer, equal to what Origin reports for its own DoubleY graph. The
  labels, line width, ticks and colours of the second edge are the fields that blocks 4 and 5 hold as blocks 2 and 3 do
  for the first edge (label size at `0x15` of block 4, width at `0x15` and tick byte `0x25` of block 5). The right title
  is the text object `YR` (header byte `0x42` is `0x84`, angle 900). The parent keeps its own axes; the top line of
  its box is written in the right layer.
- The parent's legend lists both layers: `\l(1.1) %(1.1)`, CR LF, `\l(2.1) %(2.1)` (layer, plot). Origin drew one
  legend with the entries of both layers.
- What Origin showed for `gf-doubley.opj` (one curve on each side; two on each side; a cyclic voltammogram with a right
  axis): the right axis with its own scale and title, the same X range, one legend with all curves, the right axis in
  the colour of its curve for the template's automatic colours. What differs from Origin's own file: `layer.unit`; the
  PNG of the voltammogram cuts the right tick labels at the page edge because its frame reaches 0.92 of the page.

**Bar gap.** The width of the bars in percent of the spacing, times 50, goes to `0x19` and `0x11A` of every column
record (`(100 - gap) * 50`, graphing.md 14.7). Origin drew gaps of 0, 20 and 60 percent as written. The overlap was
looked for with `set -ov`, `-vo`, `-bo`, `-ol`, `-vl` and other options and with `layer.overlap`: no record or header byte
changed, so the overlap stays in a warning. Note that two column plots written like this are drawn on top of each other
in Origin, not side by side, because the writer sets no plot group on them (byte `0x06` is 9); that was so before.

**Axis sides and colours.** The visibility bits of 12.7 are written for every axis: `AxisSide::Opposite` puts the labels,
line, ticks and colour on the second edge, `AxisSide::Hidden` shows nothing of the group. Origin reported `showAxes` 2 and
`showLabels` 2 for a top X axis, 2 and 2 for a right Y axis, 0 and 0 for hidden axes, and an automatic axis colour (-9) after
the colour of the axis line, the labels and the second edge are set to `F7 FF` (and `F7 FF FF FF` at `0x92` and `0x16A`).
The reader reads the same fields: nothing shown is a hidden axis; labels or a line only on the second edge is an
axis on the second edge (`graph.rs`, `axis_side`; the owner's five projects have no hidden or opposite axes, so their
import did not change). The title of a top axis is `XT`.

**Still in warnings** after this section (section 13.13 takes some of them off again: panels and linked Y scales, area plots, number
formats, plot groups): X offsets and the waterfall; a right layer whose X scale is not linked; the overlap of bars; the line on the near
side of an axis that is drawn on the opposite side; rotated tick labels, tick label sources and the colour of axis titles
that follow the plot.

### 13.13 Area plots, number formats, plot groups, panels [orig]

Code: `write/glayout.rs` (`curve_record`), `write/graph_tables.rs` (`CURVE_AREA`), `graph.rs` (reader), tests in
`write/graph_tests.rs` and `write/graph_feature_tests.rs`. Found on 2026-11-12 with
`tools/origin-ref/opj-make-from-lines.ps1` (builds a project from a file of LabTalk lines, saves it and exports the pictures;
with `-Open FILE` it loads a file and prints chosen values), `opj-diff-curves.py` (byte differences of curve records and
layer headers between windows) and `opj-verify-graph-features.ps1`. Files, dumps and pictures are in the git-ignored
`re-workspace/scratch/opj-gw2/`. `cargo run -p sheaf-io --example opj_write_graph_features2 -- FOLDER` writes the test files.

**Area plots.** Plot type word 204 (at `0x4C`). An Origin area record is a column-like record: connection 1 at `0x11`
(straight; the other connections are kept the same way as for lines), border width at `0x15`, the border colour at `0x0F`
and `0x16A`, the **fill colour at `0xC2`** (a four byte colour as for columns, auto = `F7 FF FF FF`), and the flag bit `0x10`
of byte `0x1E` ("fill used", as for columns). Origin's own default area is black fill with a black 0.5 pt border; setting the
line colour changes the border (`0x0F`, `0x132`, `0x16A`), setting the fill colour changes `0xC2` only. The bytes that a
line record has and an area record does not: `0x9D`, `0xDE`, `0x148`, `0x18A`, `0x174`. The four bytes at `0x17E` of an
Origin area record hold changing junk and are not written. The model's area has its fill in the symbol colour and the
border in the line colour; the reader (`read_curve`) turns type 204 into `PlotKind::Area` the same way, with a black
border when the line colour is automatic. Verified: Origin loaded `gw-area.opj` (a green area with a red 2 pt border; two
curves with their own fills), and the exported pictures show the same fills and borders. The warning for areas is gone.

**Tick label number formats.** In the tick label block of an axis group (block 2, block 4 for the second edge): bits 4 and 5
of byte `0x25` hold the format (0 decimal, 1 scientific, 2 engineering, 3 decimal with a thousands separator; Origin's
property `layer.x.label.numFormat` is this value plus one), bit 7 of the same byte says "fixed number of decimals" and the
low nibble of byte `0x26` holds that number (`layer.x.label.decPlaces`, which Origin reports as -1 when the bit is not set).
The "shown" bit of `0x26` (`0x40`) and the flag byte of a text X column (`0x25` = 1) are separate bits. Found by setting
`numFormat` 1 to 4 and `decPlaces` 0, 1, 3, 6 on separate graphs and comparing the blocks; verified by writing
`gw-fmt.opj` (X decimal 0 and Y decimal 3; X scientific 2 and Y decimal 1; automatic) and loading it in Origin: it reported
`numFormat` 1 / `decPlaces` 0 and 3, `numFormat` 2 / `decPlaces` 2 and 1, and -1 for the automatic axis, and the pictures show
`100.000`, `1.0x10^1` and plain labels respectively. The model's Decimal(n) and Scientific(n) are written and read back
(n up to 15); engineering and thousands formats are read as decimal or automatic. Date and time formats are still named in a
warning.

**Plot groups (column plots side by side).** What Origin writes for two column plots made with one plot command (a plot group):
byte `0x06` of the curve records is `0x29` for the first and `0x19` for the others (`0x09` for a plot outside any group, also for
a lone column plot). Nothing else differs from the same plots made one by one: the layer header and the per-curve blocks are
equal. The writer now sets those two values on the column plots of a layer when it has at least two of them
(`Group` in `glayout.rs`); other plots of the layer stay `0x09`. Origin then draws the bars next to each other. Things seen
with `gw-groups.opj` and patched copies of it (`re-workspace/scratch/opj-gw2/patch.py` sets single bytes of a written file):
- A group colours its members from Origin's colour list in order (black, red, green, ...) whatever the records say: the
  colours written in the records, `set -c` on the plots in Origin (it changed `0x0F`, `0x1E`, `0x132` and `0x16A` of the
  records and not the picture), and bits `0x40` and `0x80` of byte `0x06` made no difference. Only bits `0x10` (member) and `0x20`
  (first) of that byte matter. A way to keep other colours in a group (Origin's "independent" mode) was not found: `set` options
  `-xy` for 106 letter pairs with value 50 changed no byte that could hold it. The writer therefore names, in a warning,
  column plots whose colour was chosen by hand and differs from Origin's list at their place in the group; colours that come from
  the layer's palette are not named (Origin colours them itself).
- A group whose members are not next to each other in the file draws the bars on top of each other: an error plot record between
  two column curves ended the group. The writer puts the error plot records of a grouped layer after all curves. Origin then drew
  the bars side by side, but the error bars did not sit on their bars; this case is named in a warning (the picture was only
  looked at, the cause was not found).
- The overlap of bars: with the group in place `set` options `-xy` (x in v, o, g, b; y in a..z) with value 50 changed no byte of
  the records, headers or per-curve blocks except the known gap (`-vg`, `0x19` and `0x11A`). The overlap stays in a warning.
- Reading: groups are read as in 10.9 (`Plot::group`, member colours from the list). Re-saving writes them again.
Verified: Origin loaded `gw-groups.opj` (two columns, three columns, one column, two columns with an error bar) and drew
side by side bars for the first two (colours black, red, green as the list gives), a single bar set for the lone column plot.

**Panels (layers linked to the first layer).** Origin graphs made from its panel template (Vertical 2 Panel) and with
`layer.x.link = 1` / `layer.y.link = 1` set on the second layer, compared byte by byte (`re-workspace/scratch/opj-gw2/pan1.opj`,
`pan2.opj`):
- A layer is linked to an earlier one by the layer header byte `0xAB` (1-based number of the parent), as for the right layer of a
  double-Y graph; `0xFC` bit `0x80` and `0x12D` (position among the layers) are set as there. The frame stays in page units at
  `0x71`.
- "% of linked layer" (`layer.unit` = 7, and `layer.left`, `top`, `width`, `height` in percent of the parent's frame) is selected
  by the header byte `0x67` = 6 (0 gives `layer.unit` = 1, page percent). With the byte set, Origin works the place out from the
  page-unit frame: a panel of the same size right above its parent reports top -100, left 0, width and height 100; the template's
  second panel reported top -128.9. (Byte `0x2E` that 13.12 sets for the right Y layer plays no part: patching it to 0 changed
  nothing.) The pictures with and without the byte are identical. The writer sets `0x67` for panels and leaves it alone for the
  right Y layer of a double-Y graph, which stays in page percent as in 13.12.
- Whether a scale follows the parent's is not a header field: it is an object in the linked layer, like the X link of 13.12: an
  object named `X1` (X scale straight) and/or `Y1` (Y scale straight) with a 20-byte style block that holds `*`. Origin reports
  `layer.x.link` and `layer.y.link` 1 for them, 0 without. With both, Origin's file lists `Y1` before `X1`; the writer does the
  same, after the other objects of the layer (like the `X2`, `X1` pair of the right Y layer).
- A straight link means the same scale in every layer of the group, so the writer gives all of them the range that covers the data
  of all of them (X as before, Y new). Rescaling in Origin (`layer -a`, which `opj-verify-graph-features.ps1` runs before exporting
  unless `-NoRescale` is given) gives linked layers the scale of the last layer's data instead; the pictures of linked panels
  were therefore taken with `-NoRescale`.
- The model: `Layer::link` with its frame as fractions of the parent's, `x` and `y` `Straight` or `None`. A panel is written for
  every valid link that is not the right Y layer of a double-Y graph (a layer linked on a frame of its own, or whose Y axis is not
  on the right). The reader (`graph.rs`, `link_panels`) reads byte `0xAB` of every layer that `link_right_y_layers` did not link,
  makes the frame relative to the parent's (Origin's own Vertical 2 Panel reads as top -1.289 of its parent's height) and takes
  `X1` and `Y1` for the scale links. A parent that is itself linked is not followed.
- Verified: `gw-panels.opj` (`cargo run -p sheaf-io --example opj_write_graph_features2`): a stack of three panels with X linked, the
  same with a gap of 5 percent, a 3 x 1 arrangement without links, a 1 x 3 row with Y linked and a 2 x 2 with both linked.
  Origin reported `layer.link` 1 for every panel but the first, `layer.x.link` and `layer.y.link` as written, `layer.unit` 7,
  tops of -100 and -200 percent (-121.4 and -242.9 with the gap) and the frames 15, 63.33, 36.67 and 10 percent that Origin's Stack
  command gives (spec graphing.md 15.1), and the pictures show the panels in place with the shared scales.
- Still named in a warning: the hidden end tick label of a joined stack (`Axis::hide_end_label`).

**Not done: X stack offsets and waterfall.** 13.12 found no field that Origin honours for an X offset of a curve; nothing new
was tried, so these stay in warnings.

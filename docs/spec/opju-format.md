# The Origin Unicode project format (`.opju`)

Status: first study, written for the reader in `crates/io/src/opju/`. Everything here was found by
reading 19 sample projects (OriginLab's gallery projects, saved by Origin 2018 and later, signature
`CPYUA 4.3295 377`). No Origin that can open or save `.opju` was available, so nothing was checked by
writing a file and loading it. No text of the samples is quoted; the byte strings in the tests are built
by the test code.

Tags used on each statement, as in `opj-format.md`:

- **[obs N]** seen in N of the 19 files (or N records) and consistent everywhere it was looked at.
- **[inferred]** follows from the observations but was not proved.
- **[unknown]** seen but not understood, or not looked at.

The `.opj` format is described in `opj-format.md`. The `.opju` file holds the same kinds of things
(data sets that are columns, windows, a folder tree, graph and sheet properties) in a different
container, so the names of the parts are the same.

## 1. Overview

```
signature line "CPYUA 4.3295 377\n"
first record   (keywords of the project)
"PrvwOPJU" + u64 size            <- the body of the project is `size` bytes long
  body: records ... PNG pictures ... offset table of the pictures
after the body: project-level settings as XML text ("OriginStorage" trees)
```

- [obs 19] The file starts with the ASCII line `CPYUA <number> <number>` and a line feed. The first
  number was `4.3295` and the second `377` in all samples [inferred: file format version and build].
  Compare `CPYA 4.2673 378#` in `.opj`: the `U` stands for Unicode.
- [obs 19] The text `PrvwOPJU` follows the first record. The 8 bytes after it are a little-endian
  `u64` that equals the distance from the end of that field to the end of the body: in every sample the
  position `start-of-name + 16 + size` is exactly where the XML settings text begins. Between the name
  and the size field there is no other byte.
- [obs 19] The body ends with a table of 16-byte entries `(u64 offset, u64 length)`, one per preview
  picture, in file order. The offsets are positions of PNG signatures (`89 'PNG'`) inside the body; the
  lengths are the sizes of the pictures. [inferred] there is one picture per window that has one.
- [obs 19] After the body comes text: `<OriginStorage>` XML trees with settings of tools (peak
  analysis, fitting, the current folder path as `strCurPath$`, export settings). Not compressed.
  [unknown] how the project's folder tree is stored; this text only names the current folder
  (for example `/UNTITLED/` or `/<project name>/`).
- There is no whole-file compression. The high entropy of the files comes from the PNG pictures. Searching for `78 01/9C/DA` finds only bytes inside those pictures.

## 2. The first record

[obs 19] After the signature line: one byte `n`, then `n` bytes, then a line feed (`0A`).
`n` is 23 plus the length of the keywords text, in every sample. The body starts
`01 6C C0 11 01 01 80 01 08 80 09 40 00 0E 01 E4 04 00 00` (19 bytes), then one byte (keywords length + 1),
the tag `80`, one byte (keywords length), the keywords as UTF-8 (the "keywords" of the project that the
author typed, for example the plot types), and a last byte `82`. [inferred] The 19 bytes hold version
numbers (`C0 11` is `0x11C0`; it recurs in the first record of other blocks).

## 3. The stream conventions

All records are made of the same pieces. [obs 19] unless said otherwise.

**Tags and fields.** A byte with the high bit set (`0x80` to `0xFF`) is a field tag; `tag & 0x7F` is the
field number. It is followed by a length and that many bytes of value. A value may itself be a list of
fields. [inferred] Tags are not the same number for the same property in every record (the name of a
column was seen with tags `82`, `95`, `96` and `97` depending on the record), so a reader must look
at the shape of the value, not only at the tag. [unknown] the rule that picks the number.

**Lengths and long values.** A length below `0x7F` is one byte. The byte `7F` means "127 bytes follow,
and another length byte follows them". So a value of 300 bytes is `7F`, 127 bytes, `7F`, 127 bytes, the
length 46, 46 bytes. Seen in the XML text (every 128th byte is `7F`) [obs 19]; for fields that hold lists
of fields the same rule is [inferred].

**Numbers.** A count or length that does not fit a byte is a base-128 number, low group first, high bit of
a byte = "more follows" (the same as LEB128). Counts were seen as `29` (41), `8E 06` (782), `B5 0F` (1973).

**Doubles in fields.** A field with a 64-bit float value drops the zero bytes at the low end:
`F0 3F` is `1.0` (`3FF0` in the two high bytes), `11 40`-like single bytes are values such as `2.0`. [obs 19]

**Text.** Names are single-byte ASCII or UTF-8 in every sample [inferred: nothing but ASCII was seen in
names; column and window texts with other characters were not looked for]. A string is a length byte
and the bytes (no end mark). Some strings carry a trailing `00` that the length counts (the long name of a
window, section 7).

**The sentinel `CE FA`** (`0xFACE` as a little-endian 16-bit number) ends every column record. It also
can occur by chance inside numbers (a numeric block of the Trellis sample was cut short by it), so it
cannot be used to find the end of a numeric block.

## 4. Column records

One record per column (data set). Order in the file: first all column records, then the window records
(like `.opj`: data sets first, windows after). [obs 19]

```
container   80 <len> 03 00 00 01 <fields...>     the properties of the column: width, row count, ...
name field  <tag> <len> "Book59_B"               the data-set name
...
block head  <tag> 18 | u64 24 | u64 L | u64 check   26 bytes
data block  0A <kind> ...                          L bytes, the last one is CE; FA follows
```

- **Name** [obs 19]: `<window>_<column>` as in `.opj`: `Book1_B`; a sheet after the first adds
  `@n` (`Book1_B@2` is column B of the second sheet). A window name may contain an underscore, so the
  window part is found by comparing with the names of the window records. Columns called `CS_n` hold data
  behind graphs [inferred]; they have no workbook.
- **Container** [obs 19]: a field list. The fields seen for every column: row count (tag `84`, one byte),
  two doubles `1.0` (tags `83`, `86`), a width (`80 03 A0 0F 01`, 4000 in the first two bytes), the name.
  [unknown] the meaning of most other fields; the designation (X, Y, ...), format, long name, units
  and comments of the column are **not** in this record (see section 7).
- **Block head** [obs 19]: the first `u64` is always 24, the second `L` is the length of the data
  block counted from the `0A` and including the byte `CE` of the sentinel for numeric blocks. For text
  blocks `L` and `check` are both `0x2222222222222222`. [unknown] how `check` is computed (not equal to
  any simple sum or CRC tried; it differs for equal data in different columns, so it may include the name or
  an id).
- **Data block** begins with `0A` and a kind byte:
  - kind `05` is a column of numbers or of text (sections 5 and 6) [obs 19];
  - kind `04` seems to be a column of text cells that refer to a shared string table (`#_MSER_STRINGS_PSET`
    follows after the sentinel) [obs in most files]. [unknown] the layout.

## 5. Numeric blocks (kind 05)

```
0A 05 <rows> [FF FF <count> <flag>] <code> 0C <tokens...> <tail> CE
```

- `<rows>`: base-128 number, the number of rows the column holds including empty rows at the end
  (Origin keeps at least 32 rows; a column with 5 values has 32).
- Dense form [obs ~1900 columns]: `FF FF`, the number again (or the number of stored values; when it
  differs from `rows` it was the number of stored values), one byte `flag` (0 for numbers).
  Without `FF FF` (plain form, [obs ~250]): the three parts are left out and `<code>` follows `<rows>`.
- `<code>` is a base-128 number; **the number of values is `(code + 1) / 2`** [obs 1740 columns, the
  value tokens end exactly at the end of the block in all of them; the exceptions, section 5.4].
- `0C` is the type of the elements [inferred: double]. Other bytes in this place mean other kinds
  of numeric storage (section 5.4).

### 5.1 The tokens

Values are coded as changes against the value before, so a series like 105.1, 101.9, 100.8 takes
about 5 bytes per value, a constant series about nothing. [obs]

Tokens come in pairs that share one tag byte. The low nibble of the tag describes the first value of
the pair, the high nibble the second. Each nibble is:

| nibble | bytes that follow | meaning |
|---|---|---|
| `0` and `8` | 0 | same as the value before |
| `1`..`7`, `9`..`F` | `(nibble & 7) + 1`, i.e. 2 to 8 | a change, or (8 bytes, flag clear) a value |

- Bit 3 of a nibble is a flag. With 8 bytes and the flag clear (nibble `7`) the bytes are the
  value itself (little-endian `f64`). In every other case the bytes, little-endian and padded with zero
  bytes at the high end to 64 bits, are exclusive-ored with the 64 bits of the value before to give the
  value. The first value of a column has the value 0 before it. [obs: reproduces one-decimal series
  such as 31.4, 31.8, 32.5 exactly]
- The flag has no effect on a token of fewer than 8 bytes that was observed. [unknown] what it is for.
  One byte (`n = 1`) is never used; `nibble & 7 == 0` means no byte.
- The last tag byte of an odd number of values has a second nibble that is not used.
- After the last value come 0 to 10 bytes of tail before the sentinel. [obs] Seen: nothing; `xx 65`
  (two bytes where `xx` varied: `22 26 2A 2C 30 34 36 3C`); a ten-byte list that starts `88 98` (columns
  that had a change of method, section 5.4). [unknown] meaning.

### 5.2 Text in a numeric column

Origin's mixed column (numbers and text) was not seen. [unknown]

### 5.3 Missing values

A missing cell is the constant `-1.23456789E-300` (`0E 2C 13 1C FE 74 AA 81`) as in `.opj`; it takes
part in the changes like any value. [obs] A value after a missing one may come out as the marker with a few
bits changed (about `-1.12E-300`); the reader treats every nonzero value with magnitude below `1E-250` as
missing. [inferred]

### 5.4 Not understood in numeric blocks

- [obs 72 columns] The count `(code+1)/2` is larger than the number of real values: the real values end
  and a ten-byte list starting `88 98` follows, but the count says more. Example: the first 27 values
  were a clean series and the count said 38. The reader decodes the full count, so such columns have
  garbage values at the end. [unknown] what the extra count means.
- [obs ~200] Other element types: the byte after the count is not `0C` (seen `1A`, `11`, `64`, `AB`)
  and each value has its own small header. Integer-valued columns (1, 2, 3, ...) are among them. Dates are
  *not* among them: date and time columns are doubles (Julian days) in the normal form.
- [obs ~150] A one-value-per-tag form that starts `0C 00 07` (the chart-source copies, `CS_n`).
- [obs ~60] Columns of the analysis result sheets whose blocks start `05 <rows> 00 00 50`: eight-byte values
  with three-byte prefixes. [unknown]
- [obs 40] `cell://...` strings (after `FF FF <rows> 01`): references from cells to cells of other
  sheets (analysis reports). [unknown]
- Function-plot data sets (`FUNC12_3`): the start and step of a function, not values.
- [obs 207 columns, 2026-11-13] Decoding drifts inside a column: the values are a clean series up to some
  row and then stop looking like the column (in `T275K_B` of the line-graph sample, 59 values with three
  decimals, then `1019.5521105961651`, and from row 188 magnitudes above 1e250). [unknown] which token is
  misread. The reader cuts a column at the first value that is not finite or above 1e250 in magnitude and
  names the cut in a warning (`cut_implausible_tail`); the drifted values before that point are still read
  and are wrong. Counts per sample in section 9.

## 6. Text blocks (kind 05, no `0C`)

```
0A 05 <rows> { <length> <bytes> } * rows   00 ...   <second table>   CE FA
```

[obs] The strings of the rows follow in order, each a base-128 length and that many bytes; an empty cell
is `00`. A second table follows (a string with a number per entry; looks like a dictionary used for sorting
or categories). [unknown] its layout; the reader ignores it.

## 7. Window records

[obs 19] Every window that has a preview picture is a PNG followed by its record:

```
89 'PNG' ... 'IEND' AE 42 60 82     end of the picture
0A 80 <b1> <b2> 00 00 <name>       the window name, ASCII; <b1> <b2> look like a length
<fields...>
```

- **Workbooks** contain the text `ORIGIN` within their first 100 bytes [obs]. Graph records do not.
- **Long name** [obs in the workbooks that were looked at]: in the first 150 bytes after the name: `C0 FF 02 00 <x> 9F <len>`
  then the text of `len` bytes where the last is `00`. The long name is the window title that Origin shows.
- **Sheets** [obs]: each sheet of a workbook or matrix has a field `<tag> <len> <name>` followed by
  `0A 00 10 01 09 "_SEPD"`. The order of these fields is the order of the sheets. Matrix sheets are
  called `MSheet1`, ...; workbook sheets `Sheet1`, or the name the author gave.
- **Column properties**: a list of small records per column follows the sheet name, each with a
  short-name field and a text `long name \r\n units \r\n comments` (the `C,` or `H,` before the long name
  in one sample looks like the short name of the column). The text is **compressed with a scheme that
  copies earlier text** (a back reference of a length and a distance inside the same record: the sample
  shows `...Per 100,` followed by a three-byte code for `000`). [unknown] the scheme, therefore also the
  designation, number format, width and hidden flag of columns, the workbook and graph properties, the
  XML parts of the window record (user info, history script). The XML text *outside* window records
  (after the body) is not compressed.
- A window may have no preview picture [obs: several files have workbooks with columns and no window
  record found]; its columns still exist.
- [unknown] graph records: layers, axes, plots, text objects. Only the name and long name are read.
- [unknown] folder tree, window order, the window's kind flags, matrix contents.

## 8. What the reader does

`crates/io/src/opju/` (`numbers.rs`, `records.rs`, `mod.rs`):

1. Checks the signature, takes the version number from the signature line.
2. Finds window records by the picture end `IEND AE 42 60 82 0A 80`; reads name, long name, sheet names
   and the `ORIGIN` mark. A window with sheet names `MSheet..` is a matrix.
3. Finds column records by the 26-byte block head (tag byte, `18`, `24`, `L`, `check`, `0A`) and the name
   field in the 120 bytes before it; decodes numeric and text blocks.
4. Groups columns by window name and sheet number (`@n`), in file order. Columns whose window has no
   record are grouped by the text before the first underscore (not `CS`) into a workbook of their own.
5. First column of a sheet is X, the others Y (the real designations are not read, section 7).
6. Warnings name: graph windows, matrix windows, columns without a workbook, columns by reason skipped,
   rebuilt workbooks, and that the column labels and folder tree are not read.

Result on the 19 samples (numbers in `docs/spec/opju-format.md` section 9).

## 9. Reading results

Columns found by the scan / columns decoded into a workbook (the rest: matrix and graph data,
and the unsupported kinds of section 5.4):

| file | windows | columns found | decoded |
|---|---|---|---|
| 3D Bar Charts | 19 | 62 | 41 |
| 3D Scatter Ribbon and Wall Graphs | 19 | 117 | 60 |
| 3D Surface Plots | 24 | 188 | 179 |
| Area Graphs | 18 | 83 | 60 |
| Box Charts | 35 | 241 | 202 |
| Bridge Charts | 13 | 46 | 28 |
| Column and Bar Charts | 24 | 91 | 63 |
| Contour Plots | 13 | 72 | 52 |
| Financial Plots | 13 | 77 | 69 |
| Function Plots | 22 | 35 | 9 |
| Histograms | 12 | 100 | 59 |
| Line and Symbol Graphs | 55 | 384 | 352 |
| Multi-Panel Graphs | 44 | 270 | 212 |
| Multiple Axis Graphs | 18 | 76 | 40 |
| Polar Charts | 17 | 29 | 25 |
| Specialized Graphs | 47 | 155 | 120 |
| Statistical Graphs | 23 | 75 | 59 |
| Trellis Plots | 25 | 420 | 374 |
| Waterfall Plots | 15 | 176 | 165 |

Columns cut by the rule of section 5.4, and the values left out (2026-11-13): Waterfall Plots 104 / 59726,
Line and Symbol Graphs 36 / 12387, 3D Surface Plots 21 / 13592, Trellis Plots 18 / 8005, 3D Scatter Ribbon
and Wall Graphs 9 / 3527, Histograms 5 / 3389, Multi-Panel Graphs 3 / 5067, Specialized Graphs 3 / 679,
Column and Bar Charts 2 / 48, Financial Plots 2 / 13765, Multiple Axis Graphs 1 / 3498, Function Plots
1 / 330, Contour Plots 1 / 82, 3D Bar Charts 1 / 6; none in Area, Box, Bridge, Polar and Statistical.

## 10. What a writer needs (for the next agent)

Known well enough to write: the signature line, the stream conventions (section 3), column records with
numeric blocks (section 5, the encoder is in the tests), text blocks (6), window name/sheet fields (7).

Not known, and each of these decides whether Origin accepts a file:

1. The `check` value of the block head (section 4).
2. The container fields of a column (the meaning of the fields besides name and row count).
3. The compression scheme of the column property text (long names, units, comments, designation).
4. The rest of the window record (the fields after the name) and the page/layer records of graphs.
5. The body offset table and the `PrvwOPJU` size must be consistent; the pictures are optional
   [inferred, not tried].
6. The folder tree and the XML settings after the body.

A writer can produce files that Sheaf's own reader reads (the tests do this); whether Origin 2018 and
later opens them cannot be checked without such an Origin. Do not claim it does.

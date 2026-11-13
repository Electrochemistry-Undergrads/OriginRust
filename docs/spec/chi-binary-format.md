# CH Instruments binary files (`.bin`)

**Summary.** The CH Instruments (CHI) program saves a run as a binary file as well as, on request, a
text file. This document describes the layout of the binary file for three techniques: cyclic
voltammetry (CV), amperometric i-t curve (i-t) and chronopotentiometry (CP). A file is ten
length-prefixed strings, a fixed block of 0x650 bytes made of 4-byte words, and then one array of
32-bit floating point numbers (the channels alternate point by point, section 2.3). The potential of a CV and the time axis of an i-t or CP
run are **not stored**; they are rebuilt from the parameters. Every field below carries a tag:
**[checked]** (agreed with a text export of the same run, or with the physics, or with several
files), **[inferred]** (the best reading of the evidence, not proven), **[unknown]** (a value that
is present but whose meaning is not known). The reader is `crates/io/src/chi_bin.rs`.

## 1. How the layout was found

Nothing here comes from CH Instruments source code, and no CH Instruments program was started. The
evidence is:

- Eight binary files with a text export of the same run. Six are the owner's research data: four
  two-channel i-t runs (made with a CHI1040C, section 2.2), one CV and one CP (made with a CHI627E),
  each with a text export that the owner made later by converting the binary file; the text exports
  agreed with the binary files in every row of all six runs (to the rounding of the text, 4 digits).
  Two more are an older CV of the owner's made with a CHI1040C (tens of thousands of points, a run
  that was stopped early) and the sample `it.bin`/`it.txt` that ships in the `Test Data` folder of
  the CHI1040C program (3 channels, 141 points). The second form of a run made it possible to find
  the data arrays and the parameters by matching numbers. In addition the `Test Data` folder holds 20
  more binary files of other techniques without text exports, used only to see the channel layout
  (section 2.3). (None of these files is in the repository, and this document gives no number taken
  from their data.)
- Physical constraints: the size of a file equals header plus `4 x points x channels` exactly for all
  files read (the owner's six, the sample and the older CV); a CV potential runs between Low E and High E; the time of an i-t run grows by the
  sample interval; the CP potential stays between its two limits; the four i-t files share all fixed
  fields and differ in date and applied potential.
- The CHI help file (`chi627e.chm`, read as documentation only, restated here in our own words) for the
  meaning of the CP parameters: two current levels, a high and a low potential limit, a data storage
  interval, a number of segments; and for the fact that the program doubles the storage interval when
  a run would exceed its maximum number of points.
- No public description of the binary format was consulted; none is needed for what follows, and
  nothing was copied from any other implementation.

## 2. Layout

All numbers are little endian. Offsets in the table are counted from **B**, the first byte after the
tenth string. The data start at B + 0x650. Every parameter is a 32-bit float unless the type says u32.

### 2.1 Start of the file

| Offset | Type | Meaning | Tag |
|---|---|---|---|
| 0 | 4 bytes | marker `80 F2 1D 00` (present in all eight files) | [checked] as a marker; its meaning is [unknown] |
| 4 | 10 strings | each a u32 length followed by that many Latin-1 bytes | [checked] |

The ten strings, in order:

1. Short technique code: `CV`, `i-t`, `CP` [checked]. Other CHI techniques have other codes that were not seen.
2. Technique name: `Cyclic Voltammetry`, `Amperometric i-t Curve`, `Chronopotentiometry` [checked; the same words as line 2 of a text export].
3. to 9. Seven further strings, empty in nearly all files. The owner's CV has `CV` in string 3; the
   sample i-t file has `A+D=C+B` in string 7. Whether the `Header:` and `Note:` texts of a text
   export are stored here is not known: they were empty in the files that have a text export.
   [unknown]
10. A code of the form `A` plus four digits: `A4440` in the files made with a CHI1040C, `A3145` in
    those made with a CHI627 [checked as a correlation with the model number below]; its meaning
    (hardware or software revision) is [unknown]. The reader finds the end of the strings by reading
    ten strings, not by looking for this code.

### 2.2 The fixed block (offsets from B)

| Offset | Type | Meaning | Tag |
|---|---|---|---|
| 0x21c, 0x224 | u32, u32 | number of data points per channel (the two copies are always equal) | [checked] file size equals B + 0x650 + 4 x points x channels in all files read; the text export of every run that has one has that many rows |
| 0x228 .. 0x23c | 6 x u32 | year, month, day, hour, minute, second | [checked] equals the first line of the text export of the CV run |
| 0x248 | u32 | instrument model number: 1040 or 627 | [checked] 1040 in the sample and the older CV, whose text exports say `CHI1040C`, and in the owner's four i-t files, which the owner confirms were run on the CHI1040C; 627 in the owner's CV and CP, run on the CHI627E. The letter after the number is not stored anywhere found [unknown]. **Open point:** the owner's text exports of the four i-t runs say `Instrument Model: CHI627E`; a text export names the model of the program that converted it (here the CHI627E program), not the instrument that ran the run, so the word in the binary file is the better source. The reader keeps naming them CHI1040 |
| 0x254 | u32 | number of CV segments (a copy); 1 in the i-t and CP files | [inferred] |
| 0x2ec | u32 | number of data channels: 1 (CV, CP), 2 (the owner's i-t files), 3 (the sample i-t file), 8 in the other i-t file of the `Test Data` folder | [checked] by the file-size identity |
| 0x630 | f32 | layout flag: 1.0 in all of the owner's files, 0.0 in every file of the `Test Data` folder (including the sample) | [inferred] from the two groups of files only; it goes with the channel layout of section 2.3 and with `A4440` and the word 1901 at 0x350 in the older group |
| 0x3f8 | f32 | Init E in V (CV, i-t). Zero in the CP file | [checked] CV text export; sample i-t text export |
| 0x400 | f32 | CV: High E. CP: High E limit. i-t: the potential of channel 2 (E2) | [checked] High E against the CV text export; CP limit against the data (section 4); E2 against the sample i-t text [inferred] for files with more than one channel |
| 0x404 | f32 | CV: Low E. CP: Low E limit. Zero for i-t | [checked] as above |
| 0x408 | f32 | CV: scan rate in V/s | [checked] CV text export (1 V/s). In the i-t and CP files this slot holds 0.1, probably an unused default [unknown] |
| 0x410 | f32 | CV: initial scan direction, 1 = positive (`Init P/N = P`), 0 = negative (`N`). CP: 0 in the CP file, read as "cathodic first" | [checked] for `P` against the text export; [inferred] for `N` and for CP (the CP potential starts by falling) |
| 0x414 | f32 | number of segments (CV: sweep legs; CP: half cycles). A stopped run keeps the set number, not the number done | [checked] CV text export (`Segment` line); a complete CV: segments x points per leg equals the point count; a CP: the number of turning points in the data is one less than the segments |
| 0x418 | f32 | sample interval: in V for CV (0.02 in the text export), in seconds for i-t and CP (the CP "data storage interval") | [checked] for CV; for i-t the value is 0.1 in every file seen and in the sample text export, but 0x408 holds 0.1 as well, so the slot is [inferred]; for CP [inferred], see section 5 |
| 0x420 | f32 | sensitivity in A/V | [checked] for the i-t sample (0.001). [inferred] for CV: in the CV text export the words at 0x41c and 0x420 both hold 0.001, so that match does not choose between the slots; in another CV they differ and 0x420 is read, which fits its currents. It is only a record: the stored currents are already in amperes |
| 0x424 | f32 | quiet time in s | [checked] only for the value 0 (CV and i-t text exports); other values seen are plausible quiet times [inferred] |
| 0x430 | f32 | i-t run time in s. 400 in CV and CP files, probably unused | [checked] sample i-t text export; in the other i-t files the data cover just under the whole run time |
| 0x47c, 0x4ac | f32, f32 | CP cathodic and anodic current in A (equal to each other in the one CP file seen) | [inferred] the slots are the only ones holding that value; which of the two is which cannot be told because they are equal |
| 0x4b4 | f32 | sensitivity of channel 2 in A/V (i-t) | [checked] sample i-t text export (`Sens2 = 1e-003`) for the value; [inferred] for the slot |
| 0x4b8, 0x4bc | f32, f32 | CP cathodic time and anodic time in s (equal to each other in the one CP file seen) | [inferred] |
| 0x650 | f32 array | data, see 2.3 | [checked] |

Other words of the fixed block were looked at and are listed in section 5. A few, such as
0x350 (1603 or 1901) and 0x354 (2301), look like constants or version numbers.

### 2.3 Data

The data are `points x channels` 32-bit IEEE floats. With one channel they are that channel's array.
With several channels there are two layouts, told apart by the flag at 0x630:

- **Flag 1.0 (the owner's files): the channels alternate point by point**, `i1[0], i2[0], i1[1],
  i2[1], ...` [checked: every row of all eight channels of the four i-t runs agree with their text exports;
  read channel after channel they do not, and give false "pinned current" warnings]. An earlier version
  of this document and of the reader said "planar", because the only multi-channel file with a text
  export then known was the sample, below.
- **Flag 0.0 (the `Test Data` folder): channel 1 is one array, and the other channels alternate after
  it**, `i1[0..n], i2[0], i3[0], i2[1], i3[1], ...` [checked for channel 1 against the sample text, all
  141 rows; inferred for the rest: channels 2 and 3 of the sample are constant, so the text cannot tell
  the layouts apart, but read as alternating they are two constants (one value each), and read as two
  arrays each would alternate between two values; in the 8-channel i-t file of the folder channel 1 is a
  smooth decay and the other seven are smooth curves only when read with a stride of 7].

The reader uses the flag. A file with one channel is the same in both layouts.

| Technique | Channels | What the arrays are | Tag |
|---|---|---|---|
| CV | 1 | current in A (the potential is not stored) | [checked] equal to the text export of the CV run in every row, to the 4 digits of the text (largest relative difference 5e-4) |
| i-t | 1, 2 or 3 | current in A of channel 1, 2, 3 (the time is not stored) | [checked] against the sample text export (channel 1 exact to the text digits; channels 2 and 3 are constant in that sample and agree) |
| CP | 1 | potential in V (the time is not stored) | [checked] stays inside the limits at 0x400 and 0x404; the current is the set one and is not stored |

Sign convention of the current: anodic (oxidation) current is positive. Seen in the CV (the downward
sweep has negative current, the upward positive) and in the i-t files (held at 1 V the current starts
positive, held at 0.4 V negative) [inferred].

**Rebuilding the CV potential** [checked]. Point `k` (k = 0, 1, 2, ...; **the first row is Init E**) is `Init E + s k dE` with
`s = +1` for `P` and `-1` for `N` and `dE` the sample interval. The value is folded back and forth
between Low E and High E like a triangle wave: with `span = High - Low`, `u = (x - Low) mod 2 span`,
then `u = 2 span - u` if `u > span`, and the potential is `Low + u`. The first row is Init E itself: the text exports of the owner's two CV runs (CHI627E) start at
Init E, k = row index reproduces their potentials to 1e-15 V, and a four-segment run splits into scans of
2000 + 2000 rows. (An earlier version counted k from 1; that fitted the older CV, see the next
paragraph.) A limit is mirrored, not clamped: when the limits are not a whole number of steps
apart, the extreme potential is visited twice in a row (for example with limits 82.5 steps apart,
one limit once and the other twice). The formula was compared with every potential of the text
export of the CV run and agreed in every row. A second, independent way of building the potential
(walk step by step and mirror whatever passes a limit) is used in the tests. Parameters are read as
the decimal numbers they were typed as (6 significant digits), because the instrument stores some
limits one step off the nearest float (0.995 V can be stored as 0.99500006).

**Where the axes start: the CHI1040C export is the exception** [checked]. The text exports made by the
CHI1040C program (the sample `it.txt`, the older CV) start **one step in**: the first time is one sample
interval and the first potential is Init E plus a step. The exports made by the CHI627E program (all
the owner's six pairs, including the four i-t runs that were run on a CHI1040C) start at time 0 and at
Init E. Nothing in the binary file says which program will export it: the model word does not (the four
i-t files say 1040 but their exports start at 0) and the layout flag does not (the older CV has flag 1.0
and a one-step-in export). The reader follows the owner's exports; a file exported by the CHI1040C
program differs by one step on the axis, with the currents row for row the same.

**Rebuilding the time** (i-t and CP) [checked]. Point `k` (k = 0, 1, ...) is at time
`k x sample interval`, so the first row is at 0, as in the owner's text exports (previous paragraph for
the exception).

## 3. What the importer makes of a file

The workbook is made by the same code as the text importer (`chi::assemble`), so the same file gives
the same workbook either way. This was tested: the CV run and the sample i-t file, each read from
the binary file and from its text export, give sheets with the same columns, long names, units,
designations, comments, scan splits and the same warnings, and numbers that differ by at most the
rounding of the text.

- CV: sheet `Scans`, columns Potential (V, X), Current (A), Current (mA), one group per scan, comment
  `Scan k`, split at every second vertex exactly as in `chi-text-format.md` 2.2. A four-segment run
  therefore gives two scans, the second one row longer: the row that closes the second leg opens
  scan 2.
- i-t: sheet `Data`, Time (s, X), then Current (A) for one channel, or `i1`, `i2`, `i3` (A) for
  several, each followed by a Current (mA) column and, at the end, a Charge (C) column per channel
  (comment names the channel when there are several). The text importer now treats `i1/A`, `i2/A`
  columns of a text export the same way.
- CP: sheet `Data`, Time (s, X), Potential (V, Y). Nothing else (no charge).
- Header facts are returned as `meta` in the order and with the names of a text header (`Technique`,
  `Date`, `Instrument Model`, `Init E (V)`, ...), then `Instrument Code`, `Data Points`, `Channels`.
  The model is written `CHI1040` or `CHI627`, without the letter, which is not stored. The date is
  written like the text export (`Mon. DD, YYYY   hh:mm:ss`; the month spellings other than Feb., Apr.
  and Sept. are guesses). The CP parameter names are our own (taken from the help file), because no
  CP text export was available.
- Warnings: a current pinned at one value for 20 or more rows (as for text; stored values that agree
  to 4 digits, the digits a text export would show, count as equal, because the instrument's limit
  value wiggles in the last digits); a run that ends before its set run time; a file whose data are
  cut short (one channel) or followed by extra bytes; a CP potential outside its limits; stored
  values that are not finite numbers.
- Refused with a message that names the file and the technique: other techniques; a file without the
  marker; a file that ends inside the strings or the parameter block; a multi-channel file whose
  arrays are cut short; CV parameters that do not make a sweep (High E not above Low E, Init E outside
  the range, sample interval not positive).
- `import_file` decides by the first four bytes, not by the extension, so the app's import dialog,
  `sheaf import`, `sheaf info` and `sheaf plot` take `.bin` files (and a binary file with any other
  name). Choosing "Delimited text" for such a file gives the usual "binary file" message.

## 4. Physical checks (and which were done on what)

On synthetic files made by a test-only writer of this layout (tests in `chi_bin/tests.rs`):

- CV potential equals an independent step-and-mirror simulation for four parameter sets, with limits
  a whole number of steps apart and not, in both directions, and stays between the limits.
- First row Init E; limits reached at the expected rows; the repeated extreme;
  vertices 83 and 82 rows apart; scan sizes; a four-segment run gives 2000 + 2000 rows.
- i-t: time spacing equals the sample interval; the running charge of a linear current equals the
  exact integral; the bytes of a two-channel run alternate point by point and the old three-channel
  layout keeps channel 1 apart; the time starts at 0 in both layouts; clipped current warning names the
  channel; early end of run; the Header text is kept.
- CP: the potential stays inside its limits; three turning points for four segments; the set current
  and limits come back in the header list; a potential beyond a limit warns.
- No prefix of a valid file and no single byte flipped makes the reader panic.

On local files (the tests skip when none are present; they name no file and state no number from
the data):

- Every local binary file of a known technique is read without error. For CV the rebuilt sweep stays
  between the header's limits; for i-t the time grows by the sample interval from 0 on;
  for CP the potential stays between its two limits.
- Every local binary file that has its text export beside it gives the same workbook from both:
  same columns, scans, warnings, Header text, and numbers within the rounding of the text. (Exports of
  the CHI1040C program, which start one step in, are compared on their currents only.) Run on the
  owner's six pairs, the sample and the older CV: all pass.
- Checked by hand on the owner's six files: a complete CV has segments x points per leg points, the
  stored sweep covers exactly Low E to High E, the current of upward legs is positive; the i-t time
  of the last point equals the run time to within 0.1 percent; the CP potential reaches each limit to
  within 1 mV and turns one time fewer than its segment count.
- Looking at the figures (`sheaf plot`): the CV looks like a CV with oxidation and reduction waves
  and nearly overlapping scans; the i-t files are current decays; the CP is a charge-discharge curve
  with plateaus. This does not prove the numbers but excludes gross errors (a wrong channel or a
  wrong waveform would not look like this).

Not verified: the layout of three or more channels in files with flag 1.0 (the owner's have two); the
layout flag as the cause of the channel layout (two groups of files only); every parameter marked
[inferred] or [unknown].

## 5. Unknowns

- Strings 4 to 9, and where `Note:` text is stored. The third string holds the `Header:` text (`CV` in
  the owner's CV and its text export [checked]); the reader keeps it as the metadata entry `Header`. The
  fourth might be the note [unknown]; all files seen have it empty. The marker word's meaning, and the
  code string 10.
- The CP sample interval. In the one CP file seen, the slot at 0x418 and the other interval-like
  slot at 0x408 hold different values; 0x418 gives the more plausible half-cycle length, but the
  other cannot be excluded. The CHI program also doubles the interval when a run would hold too
  many points, so the stored value might not be the one in force. This is the main thing a text export
  of the same file would settle.
- Which CP current slot is cathodic, and the sign convention of the stored CP currents.
- The initial-polarity word for CP, the CV `Final E` (word at 0x3fc was 0 in one CV and equal to Init E
  in the other), E3 and Sens3 of three-channel runs (words at 0x4b0 and 0x4ec change from file to
  file and do not clearly belong to a channel), and the sensitivity slot of CV: 0x420 is assumed
  for all techniques; the word at 0x41c holds 0.001 in both CV files and 0.1 in the others.
- Whether the date is the start or the end of the run. For the i-t files the file's modification
  time is minutes after the stored time although the runs last hours, which suggests the stored
  time is the end of the run [inferred].
- The instrument model letter (CHI627**E**, CHI1040**C**).
- Many flag words (offsets 0x28, 0x90, 0xe4 to 0xf0, 0x140 to 0x188, 0x208, 0x20c, 0x250, 0x258,
  0x25c, 0x268, 0x28c to 0x298, 0x2d4 to 0x2e8, 0x310 to 0x344, 0x350 to 0x378, many words from
  0x40c to 0x630) that are constant, or differ between files without a clear connection with the
  parameters. They are probably other techniques' parameters and defaults.
- Everything about other techniques (linear sweep, pulse and square wave voltammetry, impedance,
  open circuit, bulk electrolysis, ...): the layout of the block and the arrays is probably similar,
  but nothing here was checked, so such files are refused rather than guessed.
- Files with several data blocks, and runs that were continued or paused.

## 6. What a text export of the same files would settle

Done since: items 1 (CP time column and interval), 2 (CV potential and header) and 3 (two-channel
i-t) were settled by the owner's text exports of 2026-11 (sections 2.2, 2.3); item 4 (Header text) in
part (`CV` found; the Note field is still unknown). The list below is the original one.

For the owner's own runs, one text export from the CHI program per technique (File, Save As, text,
of the same run, without changing anything) would let us check:

1. **The CP file**: the `Time/sec` column. It gives the true sample interval, shows whether the first row is at one interval, and gives the printed CP header
   (parameter names, which current is cathodic, signs, initial polarity, `Segment`). Most valuable.
2. **The CV file**: the Potential column (first point, the exact rebuilt grid, the
   vertex rows), the header (`Init P/N`, `Scan Rate`, `Sensitivity`, `Quiet Time`, `Final E` if
   printed), the `Header:`/`Note:` lines, and the column names. This checks the sensitivity slot
   and the claim that the stored current is already in amperes for this run.
3. **One two-channel i-t file**: the column header (`Time/s, i1/A, i2/A`?), `E2`/`Sens2` lines,
   the first time value, whether the run time and the point count agree with the text rows, and which
   channel is which. It also tells how the clipped values are printed.
4. **A run whose Header and Note fields hold some text**, to find where the strings are stored.
5. **The date line of any of them**, together with a note of when the run started, to settle start
   or end time.
6. To extend the reader: a binary file and its text export of each other technique used (linear
   sweep, differential pulse, square wave, impedance, open circuit potential-time).

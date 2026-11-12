<#
.SYNOPSIS
  Builds a small Origin project from closed-form data that we invent here, and saves it as
  an .opj file, so the reader of project files can be compared with Origin on several kinds
  of column (numbers, text, mixed text and numbers, missing values, dates and times, a column
  formula, designations, several sheets, an empty workbook, a hidden workbook).

.DESCRIPTION
  Origin is started hidden through COM (system-wide lock, like run-labtalk.ps1). Everything is
  done with LabTalk commands, because the COM collections (WorksheetPages, Columns) report
  zero items in a hidden instance. The project is saved to -Out (a new file; an existing file
  is never overwritten). Compare it with opj-dump-cells.ps1 on that file and `opj_dump --cells`.

  The book SynFmt has one column per display-format code 0 to 12, each filled with the same
  day numbers, so that the listing shows which code means date, time and so on.

  -NoLock is for a calling script that already holds the lock Global\SheafOriginReference.

.EXAMPLE
  .\opj-make-test-project.ps1 -Out C:\scratch\synthetic.opj
#>
param([Parameter(Mandatory = $true)][string]$Out, [switch]$NoLock)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $Out) { throw "Refusing to overwrite $Out" }
$Out = [IO.Path]::GetFullPath($Out)

$lock = $null
if (-not $NoLock) {
  $lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
  try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(60)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
  if (-not $held) { throw 'Another Origin reference run held the lock for more than 60 minutes.' }
}
$o = $null
try {
  $o = New-Object -ComObject Origin.Application
  function Lt([string]$s) { [void]$o.Execute($s) }
  [void]$o.NewProject()

  function NewBook([string]$name, [int]$cols) {
    Lt "newbook name:=$name sheet:=1 option:=lsname;"
    Lt "win -a $name; wks.ncols = $cols;"
  }
  # Fills column $col rows 1..$n with an expression in the row number r. Plain `col(1) = ... i ...`
  # assignments left most cells empty in a hidden instance, so every value is set one cell at a time.
  function Fill([int]$col, [int]$n, [string]$expr) {
    Lt "wks.nrows = $n; loop(r, 1, $n) { cell(r, $col) = $expr; }"
  }
  function Label([int]$col, [string]$long, [string]$units, [string]$comments) {
    Lt "wks.col$col.lname`$ = `"$long`"; wks.col$col.unit`$ = `"$units`"; wks.col$col.comment`$ = `"$comments`";"
  }

  # --- SynNum: numbers, designations, missing values ----------------------------------------
  # LabTalk column type codes: 1 Y, 2 disregard, 3 Y error, 4 X, 5 label, 6 Z, 7 X error.
  NewBook 'SynNum' 8
  Fill 1 40 '0.25 * (r - 1) - 3'
  Label 1 'abscissa' 'V' 'closed form 0.25*i-3'
  Fill 2 40 'sin(r) * 1e-7 + r * 1.0000000001'
  Label 2 'sine plus ramp' 'u' 'double with a fine structure'
  Fill 3 40 '0.5 * r - 7.25'
  Label 3 'half steps' 'u' 'y error column'
  Fill 4 40 '311 * r - 6000'
  Label 4 'integers' 'u' 'x error column'
  Fill 5 40 '1.5e-12 * r'
  Label 5 'tiny' 'u' 'z column'
  Fill 6 40 '1e15 * r'
  Label 6 'huge' 'u' 'label column'
  Fill 7 40 '-r * 0.1'
  Label 7 'negative' 'u' 'disregard column'
  Fill 8 40 'r'
  Label 8 'plain' '' ''
  Lt 'wks.col1.type = 4; wks.col3.type = 3; wks.col4.type = 7; wks.col5.type = 6; wks.col6.type = 5; wks.col7.type = 2;'
  Lt 'cell(6, 2) = NANUM; cell(7, 2) = NANUM; cell(40, 8) = NANUM;'

  # --- SynFmt: one column per display format code --------------------------------------------
  NewBook 'SynFmt' 13
    for ($k = 1; $k -le 13; $k++) {
    Fill $k 12 '40000.5 + 31.25 * (r - 1)'
    Lt "wks.col$k.format = " + ($k - 1) + ";"
    Label $k ("format code " + ($k - 1)) '' 'same day numbers in every column'
  }

  # --- SynMix: text, mixed, formula, three sheets ---------------------------------------------
  NewBook 'SynMix' 5
  $accented = 'caf' + [char]0xE9 + ' ' + [char]0xB0 + 'C ' + [char]0xB5 + 'm'
  for ($i = 1; $i -le 12; $i++) {
    $text = 'row ' + ($i - 1) + ' ' + ('x' * (($i - 1) % 5))
    if ($i -eq 4) { $text = $accented }
    Lt ("cell($i, 1) = `"$text`";")
  }
  Label 1 'plain text' '' 'text column with non-ASCII characters'
  for ($i = 1; $i -le 12; $i++) {
    if (($i - 1) % 3 -eq 0) { Lt ("cell($i, 2) = `"n/a " + ($i - 1) + "`";") } else { Lt ("cell($i, 2) = " + (0.125 * ($i - 1)).ToString([Globalization.CultureInfo]::InvariantCulture) + ";") }
  }
  Label 2 'mixed' 'a.u.' 'text and numbers'
  Fill 3 12 '(r - 1) * (r - 1)'
  Label 3 'squares' 'm^2' 'source for the formula column'
  Fill 4 12 '(r - 1) * (r - 1) * 2 + 1'
  Label 4 'formula result' 'm^2' 'col(4) = col(3)*2+1'
  Lt 'newsheet name:="Second" cols:=2;'
  Fill 1 6 '100 + (r - 1) * 0.5'
  Fill 2 6 '110 + (r - 1) * 0.5'
  Label 1 'sheet 2 col 1' 'unit' 'comment 1/0'
  Label 2 'sheet 2 col 2' 'unit' 'comment 1/1'
  Lt 'newsheet name:="Third one" cols:=3;'
  Fill 1 7 '200 + (r - 1) * 0.5'
  Fill 2 7 '210 + (r - 1) * 0.5'
  Fill 3 7 '220 + (r - 1) * 0.5'
  Label 1 'sheet 3 col 1' 'unit' 'comment 2/0'
  Label 2 'sheet 3 col 2' 'unit' 'comment 2/1'
  Label 3 'sheet 3 col 3' 'unit' 'comment 2/2'

  # --- SynEmpty: no data; SynHidden: hidden -------------------------------------------------
  NewBook 'SynEmpty' 2
  NewBook 'SynHidden' 2
  Fill 1 3 '1.5 + (r - 1)'
  Lt 'win -h 1;'

  if (-not $o.Save($Out)) { throw "Origin could not save $Out" }
  "saved $Out"
}
finally {
  if ($null -ne $o) {
    try { [void]$o.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($o)
    Start-Sleep -Seconds 2
  }
  if ($null -ne $lock) {
    $lock.ReleaseMutex()
    $lock.Dispose()
  }
}

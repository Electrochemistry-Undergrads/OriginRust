<#
.SYNOPSIS
  Builds a small Origin project from closed-form data that we invent here, and saves it as
  an .opj file, so the reader of project files can be compared with Origin on every kind of
  column (number types, text, mixed text and numbers, missing values, dates and times,
  formulas, designations, several sheets, an empty workbook, a hidden workbook).

.DESCRIPTION
  Origin is started hidden through COM (system-wide lock, like run-labtalk.ps1). The project
  is saved to -Out (a new file; an existing file is never overwritten). Compare it with
  opj-dump-cells.ps1 on that file and `opj_dump --cells`.

.EXAMPLE
  .\opj-make-test-project.ps1 -Out C:\scratch\synthetic.opj
#>
param([Parameter(Mandatory = $true)][string]$Out)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $Out) { throw "Refusing to overwrite $Out" }
$Out = [IO.Path]::GetFullPath($Out)

$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 30 minutes.' }
$o = $null
try {
  $o = New-Object -ComObject Origin.Application
  [void]$o.NewProject()

  function NewBook([string]$name, [int]$cols) {
    [void]$o.Execute("newbook name:=$name sheet:=1 option:=lsname;")
    $page = $o.WorksheetPages.Item($name)
    $sheet = $page.Layers.Item(0)
    while ($sheet.Columns.Count -lt $cols) { [void]$sheet.Columns.Add() }
    return $page
  }
  function Label($col, [string]$long, [string]$units, [string]$comments) {
    $col.LongName = $long; $col.Units = $units; $col.Comments = $comments
  }

  # --- Book 1: numbers of every storage type, designations, missing values -----------------
  $n = 40
  $page = NewBook 'SynNum' 9
  $sheet = $page.Layers.Item(0)
  $formats = @(@(0, 'double'), @(32, 'float'), @(33, 'short'), @(34, 'long'), @(35, 'char'), @(38, 'byte'), @(39, 'ushort'), @(40, 'ulong'))
  # column 0: X
  $x = New-Object 'double[]' $n
  for ($i = 0; $i -lt $n; $i++) { $x[$i] = 0.25 * $i - 3.0 }
  $c = $sheet.Columns.Item(0); $c.Type = 3; [void]$c.SetData($x, 0)
  Label $c 'abscissa' 'V' 'closed form 0.25*i-3'
  for ($k = 0; $k -lt $formats.Count; $k++) {
    $c = $sheet.Columns.Item($k + 1)
    $c.DataFormat = $formats[$k][0]
    $v = New-Object 'double[]' $n
    for ($i = 0; $i -lt $n; $i++) {
      switch ($formats[$k][1]) {
        'double' { $v[$i] = [Math]::Sin($i) * 1e-7 + $i * 1.0000000001 }
        'float' { $v[$i] = 0.5 * $i - 7.25 }
        'short' { $v[$i] = 311 * $i - 6000 }
        'long' { $v[$i] = 70001 * $i - 1000000 }
        'char' { $v[$i] = $i - 20 }
        'byte' { $v[$i] = 6 * $i }
        'ushort' { $v[$i] = 1500 * $i }
        'ulong' { $v[$i] = 100000007 * $i }
      }
    }
    [void]$c.SetData($v, 0)
    Label $c ('type ' + $formats[$k][1]) 'u' ('column of storage type ' + $formats[$k][1])
  }
  # designations: 2 = yErr, 6 = xErr, 5 = Z, 4 = label, 1 = disregard
  $sheet.Columns.Item(2).Type = 2
  $sheet.Columns.Item(3).Type = 6
  $sheet.Columns.Item(4).Type = 5
  $sheet.Columns.Item(5).Type = 4
  $sheet.Columns.Item(6).Type = 1
  # missing values in column 8 (ulong): none possible, so use column 1 (double) via clearing cells
  [void]$o.Execute('[SynNum]1!wks.col2[5]=0/0; [SynNum]1!cell(8,2)=0/0;')

  # --- Book 2: text, mixed, dates, times, formula, 3 sheets --------------------------------
  $page = NewBook 'SynMix' 7
  $sheet = $page.Layers.Item(0)
  $t = New-Object 'object[]' 12
  for ($i = 0; $i -lt 12; $i++) { $t[$i] = 'row ' + $i + ' ' + ('x' * ($i % 5)) }
  $t[3] = 'caf' + [char]0xE9 + ' ' + [char]0xB0 + 'C ' + [char]0xB5 + 'm'
  $c = $sheet.Columns.Item(0); $c.DataFormat = 1; [void]$c.SetData($t, 0)
  Label $c 'plain text' '' 'text column with non-ASCII characters'
  $m = New-Object 'object[]' 12
  for ($i = 0; $i -lt 12; $i++) { if ($i % 3 -eq 0) { $m[$i] = 'n/a ' + $i } else { $m[$i] = [double](0.125 * $i) } }
  $c = $sheet.Columns.Item(1); $c.DataFormat = 9; [void]$c.SetData($m, 0)
  Label $c 'mixed' 'a.u.' 'text and numbers'
  $d = New-Object 'double[]' 12
  for ($i = 0; $i -lt 12; $i++) { $d[$i] = 40000.5 + 31.25 * $i }
  $c = $sheet.Columns.Item(2); $c.DataFormat = 3; [void]$c.SetData($d, 0)
  Label $c 'date' '' 'date column'
  $c = $sheet.Columns.Item(3); $c.DataFormat = 2
  $tm = New-Object 'double[]' 12
  for ($i = 0; $i -lt 12; $i++) { $tm[$i] = ($i * 0.0625) % 1.0 }
  [void]$c.SetData($tm, 0)
  Label $c 'time' '' 'time of day'
  $c = $sheet.Columns.Item(4)
  $v = New-Object 'double[]' 12
  for ($i = 0; $i -lt 12; $i++) { $v[$i] = $i * $i }
  [void]$c.SetData($v, 0)
  Label $c 'squares' 'm^2' 'source for the formula column'
  [void]$o.Execute('[SynMix]1!col(F) = col(E) * 2 + 1;')
  Label $sheet.Columns.Item(5) 'formula result' 'm^2' 'col(F) = col(E)*2+1'
  # extra sheets, with their own columns
  [void]$o.Execute('[SynMix]1!newsheet name:="Second" cols:=2;')
  [void]$o.Execute('[SynMix]1!newsheet name:="Third one" cols:=3;')
  for ($s = 1; $s -le 2; $s++) {
    $sh = $page.Layers.Item($s)
    for ($k = 0; $k -lt $sh.Columns.Count; $k++) {
      $v = New-Object 'double[]' (5 + $s)
      for ($i = 0; $i -lt $v.Length; $i++) { $v[$i] = $s * 100 + $k * 10 + $i * 0.5 }
      [void]$sh.Columns.Item($k).SetData($v, 0)
      Label $sh.Columns.Item($k) ("sheet $($s + 1) col $k") 'unit' ("comment $s/$k")
    }
  }

  # --- Book 3: empty; Book 4: hidden ---------------------------------------------------------
  [void](NewBook 'SynEmpty' 2)
  $page = NewBook 'SynHidden' 2
  [void]$page.Layers.Item(0).Columns.Item(0).SetData([double[]](1.5, 2.5, 3.5), 0)
  [void]$o.Execute('win -a SynHidden; win -h 1;')

  if (-not $o.Save($Out)) { throw "Origin could not save $Out" }
  "saved $Out"
}
finally {
  if ($null -ne $o) {
    try { [void]$o.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($o)
    Start-Sleep -Seconds 2
  }
  $lock.ReleaseMutex()
  $lock.Dispose()
}

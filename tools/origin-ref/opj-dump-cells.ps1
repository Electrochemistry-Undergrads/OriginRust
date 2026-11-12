<#
.SYNOPSIS
  Opens an Origin project read-only in a hidden Origin instance (through COM), walks every
  workbook and sheet and writes a listing of the columns and a sample of the cell values.

.DESCRIPTION
  The listing has the same format as `cargo run -p sheaf-io --example opj_dump -- FILE --cells`
  (see the header of crates/io/examples/opj_dump.rs), so the two can be compared with
  opj-compare-cells.ps1:

    BOOK   <window name>  <long name>
    SHEET  <sheet name>
    COL    <index> <short name> <long name> <units> <comments> <designation> <used rows>
    CELL   <index> <row> <N number | T text | M missing> <value>

  Row and column indices start at 0. "Used rows" is the position of the last cell that is not
  missing, plus one. The sampled rows are the first three, the last three and twenty spread
  evenly over the used rows. Numbers are written with 17 significant digits' worth of
  round-trip text ("R" format, invariant culture). Backslash, tab, line feed and carriage
  return in text are written as backslash plus \, t, n or r.
  Two more line kinds exist only here: "# ..." for notes and "ERR ..." when Origin refused a
  call. The Origin column type is written as the designation name Origin uses for the same
  number that our reader uses (0 Y, 1 Disregard, 2 YError, 3 X, 4 Label, 5 Z, 6 XError,
  7 Group, 8 Subject).

  The project is never saved: it is loaded read-only and Origin is closed without saving.
  Like run-labtalk.ps1 this takes the system-wide lock, so only one Origin instance runs at a
  time; a second copy of the script waits (up to 30 minutes).

  Origin refuses to open a project from a protected folder such as C:\Program Files read-only
  silently: it shows a message box that nobody can answer in a hidden instance, and the run
  hangs. For such projects use -CopyFirst: the file is copied to a temporary folder (the
  original is only read) and the copy is opened.

.EXAMPLE
  .\opj-dump-cells.ps1 -Project C:\data\my.opj -Out C:\scratch\my.origin.txt
  .\opj-dump-cells.ps1 -Project "C:\Program Files\OriginLab\Origin2016\Samples\Simple XY.opj" -Out s.txt -CopyFirst
#>
param(
  [Parameter(Mandatory = $true)][string]$Project,
  [Parameter(Mandatory = $true)][string]$Out,
  [switch]$CopyFirst,
  [int]$AllRowsLimit = 0
)
$ErrorActionPreference = 'Stop'
$inv = [Globalization.CultureInfo]::InvariantCulture
$MissingBits = [BitConverter]::DoubleToInt64Bits(-1.23456789e-300)
# LabTalk's wks.colN.type codes, written as the designation names our reader uses.
$LabTalkTypes = @{ 1 = 'Y'; 2 = 'Disregard'; 3 = 'YError'; 4 = 'X'; 5 = 'Label'; 6 = 'Z'; 7 = 'XError'; 8 = 'Group'; 9 = 'Subject' }

function Esc([string]$s) {
  if ($null -eq $s) { return '' }
  $s.Replace('\', '\\').Replace("`t", '\t').Replace("`n", '\n').Replace("`r", '\r')
}

function IsMissing($v) {
  if ($null -eq $v -or $v -is [DBNull]) { return $true }
  if ($v -is [string]) { return $v.Length -eq 0 }
  $d = [double]$v
  return [double]::IsNaN($d) -or [BitConverter]::DoubleToInt64Bits($d) -eq $MissingBits
}

function SampleRows([int]$used) {
  if ($used -gt 0 -and $used -le $AllRowsLimit) { return 0..($used - 1) }
  $rows = New-Object 'System.Collections.Generic.SortedSet[int]'
  for ($r = 0; $r -lt [Math]::Min(3, $used); $r++) { [void]$rows.Add($r) }
  for ($r = [Math]::Max(0, $used - 3); $r -lt $used; $r++) { [void]$rows.Add($r) }
  if ($used -gt 0) { for ($k = 0; $k -lt 20; $k++) { [void]$rows.Add([int][Math]::Floor($k * ($used - 1) / 19)) } }
  return $rows
}

$source = (Resolve-Path -LiteralPath $Project).Path
$tempDir = $null
if ($CopyFirst) {
  $tempDir = Join-Path ([IO.Path]::GetTempPath()) ('opjdump-' + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Path $tempDir | Out-Null
  $opened = Join-Path $tempDir ([IO.Path]::GetFileName($source))
  Copy-Item -LiteralPath $source -Destination $opened
} else {
  $opened = $source
}

$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 30 minutes.' }
$origin = $null
$w = New-Object IO.StreamWriter($Out, $false, (New-Object Text.UTF8Encoding($false)))
$w.NewLine = "`n"
try {
  $origin = New-Object -ComObject Origin.Application
  # The COM collections (WorksheetPages, Pages) stay empty after Load in a hidden instance, so the
  # structure and the cells are read with LabTalk (results come back through LTStr and LTVar).
  # GetWorksheet answered only for some sheets (it returned an error number for the others).
  function Lt([string]$script) { [void]$origin.Execute($script) }
  function LtS([string]$script, [string]$var) { Lt $script; return [string]$origin.LTStr($var) }
  function LtN([string]$script, [string]$var) { Lt $script; return [double]$origin.LTVar($var) }
  # Text of a cell as Origin shows it; $null when Origin refuses (row past the column's data).
  function CellText([int]$r, [int]$c) {
    $okc = $origin.Execute("string cs`$ = cell($r, $c)`$;")
    if (-not $okc) { return $null }
    return [string]$origin.LTStr('cs$')
  }
  $ok = $origin.Load($opened, $true)
  $w.WriteLine("# project: loaded=$ok")
  $nb = [int](LtN 'double nb = 0; doc -e W { nb++; }' 'nb')
  $w.WriteLine("# workbooks: $nb")
  $names = @()
  for ($k = 1; $k -le $nb; $k++) {
    Lt "double kk = 0; string bn`$ = `"`"; doc -e W { kk++; if (kk == $k) { bn`$ = `"%H`"; } }"
    $names += [string]$origin.LTStr('bn$')
  }
  foreach ($book in $names) {
    Lt "win -a $book;"
    $long = LtS 'string bl$ = page.longname$;' 'bl$'
    $w.WriteLine("BOOK`t" + (Esc $book) + "`t" + (Esc $long))
    $nl = [int](LtN 'double nl = page.nlayers;' 'nl')
    for ($s = 1; $s -le $nl; $s++) {
      Lt "win -a $book; page.active = $s;"
      $sname = LtS 'string sn$ = wks.name$;' 'sn$'
      $w.WriteLine("SHEET`t" + (Esc $sname))
      $nc = [int](LtN 'double nc = wks.ncols;' 'nc')
      $rowsStored = [int](LtN 'double mr = wks.nrows;' 'mr')
      $w.WriteLine("# sheet $s columns=$nc stored rows=$rowsStored")
      for ($i = 0; $i -lt $nc; $i++) {
        $c1 = $i + 1
        try {
          Lt "win -a $book; page.active = $s;"
          $short = LtS "string a`$ = wks.col$c1.name`$;" 'a$'
          $lname = LtS "string a`$ = wks.col$c1.lname`$;" 'a$'
          $units = LtS "string a`$ = wks.col$c1.unit`$;" 'a$'
          $comm = LtS "string a`$ = wks.col$c1.comment`$;" 'a$'
          $type = [int](LtN "double ty = wks.col$c1.type;" 'ty')
          $role = $LabTalkTypes[$type]
          if ($null -eq $role) { $role = "Type$type" }
          # Used rows: the last cell that holds a number or text. cell(r,c)$ makes the script fail
          # for rows past the end of a column's data, so numbers are scanned first (they need no
          # text), then the text length is found by bisection and scanned for text below it.
          Lt ("double us = 0; for (us = $rowsStored; us > 0; us--) { if (cell(us, $c1) != NANUM) { break; } }")
          $used = [int]$origin.LTVar('us')
          if ($null -ne (CellText 1 $c1)) {
            $lo = 1; $hi = $rowsStored
            while ($lo -lt $hi) {
              $mid = [int][Math]::Ceiling(($lo + $hi) / 2)
              if ($null -ne (CellText $mid $c1)) { $lo = $mid } else { $hi = $mid - 1 }
            }
            for ($r = $lo; $r -gt $used; $r--) {
              $t = CellText $r $c1
              if ($t -ne '' -and $t -ne '--') { $used = $r; break }
            }
          }
          $w.WriteLine("COL`t$i`t" + (Esc $short) + "`t" + (Esc $lname) + "`t" + (Esc $units) + "`t" + (Esc $comm) + "`t$role`t$used")
          foreach ($r in (SampleRows $used)) {
            $r1 = $r + 1
            Lt "double cv = cell($r1, $c1);"
            $v = [double]$origin.LTVar('cv')
            $t = ''
            if (IsMissing $v) { $t = CellText $r1 $c1 }
            if (-not (IsMissing $v)) { $w.WriteLine("CELL`t$i`t$r`tN`t" + $v.ToString('R', $inv)) }
            elseif ($t.Length -gt 0 -and $t -ne '--') { $w.WriteLine("CELL`t$i`t$r`tT`t" + (Esc $t)) }
            else { $w.WriteLine("CELL`t$i`t$r`tM`t") }
          }
        } catch {
          $w.WriteLine("ERR`tcolumn $i of sheet '$sname': " + ($_.Exception.Message -replace "[\r\n]+", ' '))
        }
      }
    }
  }
}
finally {
  $w.Dispose()
  if ($null -ne $origin) {
    # The project was opened read-only and nothing was changed; Exit does not save.
    try { [void]$origin.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)
    Start-Sleep -Seconds 2
  }
  $lock.ReleaseMutex()
  $lock.Dispose()
  if ($null -ne $tempDir) { Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue }
}

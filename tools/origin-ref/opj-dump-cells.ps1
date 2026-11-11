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
  [switch]$CopyFirst
)
$ErrorActionPreference = 'Stop'
$inv = [Globalization.CultureInfo]::InvariantCulture
$MissingBits = [BitConverter]::DoubleToInt64Bits(-1.23456789e-300)
$Roles = @{ 0 = 'Y'; 1 = 'Disregard'; 2 = 'YError'; 3 = 'X'; 4 = 'Label'; 5 = 'Z'; 6 = 'XError'; 7 = 'Group'; 8 = 'Subject' }

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
  $ok = $origin.Load($opened, $true)
  $w.WriteLine("# project: loaded=$ok")
  $pages = $origin.WorksheetPages
  $w.WriteLine("# workbooks: " + $pages.Count)
  for ($p = 0; $p -lt $pages.Count; $p++) {
    $page = $pages.Item($p)
    $w.WriteLine("BOOK`t" + (Esc $page.Name) + "`t" + (Esc $page.LongName))
    $layers = $page.Layers
    for ($s = 0; $s -lt $layers.Count; $s++) {
      $sheet = $layers.Item($s)
      $w.WriteLine("SHEET`t" + (Esc $sheet.Name))
      $cols = $sheet.Columns
      for ($i = 0; $i -lt $cols.Count; $i++) {
        $c = $cols.Item($i)
        try {
          $data = $c.GetData(5)
          $n = if ($null -eq $data) { 0 } else { $data.Length }
          $used = $n
          while ($used -gt 0 -and (IsMissing $data[$used - 1]) ) { $used-- }
          $role = $Roles[[int]$c.Type]
          if ($null -eq $role) { $role = "Type" + [int]$c.Type }
          $w.WriteLine("COL`t$i`t" + (Esc $c.Name) + "`t" + (Esc $c.LongName) + "`t" + (Esc $c.Units) + "`t" + (Esc $c.Comments) + "`t$role`t$used")
          $w.WriteLine("# col $i stored rows=$n format=" + [int]$c.DataFormat)
          foreach ($r in (SampleRows $used)) {
            $v = $data[$r]
            if (IsMissing $v) { $w.WriteLine("CELL`t$i`t$r`tM`t") }
            elseif ($v -is [string]) { $w.WriteLine("CELL`t$i`t$r`tT`t" + (Esc $v)) }
            else { $w.WriteLine("CELL`t$i`t$r`tN`t" + ([double]$v).ToString('R', $inv)) }
          }
        } catch {
          $w.WriteLine("ERR`tcolumn $i of sheet '" + $sheet.Name + "': " + ($_.Exception.Message -replace "[\r\n]+", ' '))
        }
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($c)
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

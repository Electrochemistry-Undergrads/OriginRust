<#
.SYNOPSIS
  Opens project files in a hidden Origin (through COM) and lists the folder tree with the
  windows of each folder, and for every workbook sheet the column formulas and their recalculate
  mode, as Origin itself reports them. Used to check what the .opj writer stores for folders and
  "set column values" formulas (docs/spec/opj-format.md, sections 13.10 and 13.11).

.DESCRIPTION
  Output, tab separated, one line per item:

    FOLDER  <path>  <folder name>
    PAGE    <folder path>  <window name>  <type: 2 workbook, 3 matrix, 5 graph, ...>
    COL     <window name>  <sheet number>  <column number>  <short name>  <recalculate: 0 none, 1 auto, 2 manual>  <formula>

  Only columns that have a formula text or a recalculate mode other than 0 get a COL line. The
  project is loaded from a temporary copy and never saved. The run takes the system-wide lock
  Global\SheafOriginReference, is killed by a watchdog after -TimeoutSec seconds (a modal dialog in
  a hidden Origin would hang it) and only ever closes the Origin instance it started.

.EXAMPLE
  .\opj-verify-features.ps1 -Project C:\scratch\written.opj -Out C:\scratch\written.features.txt
#>
param(
  [Parameter(Mandatory = $true)][string]$Project,
  [Parameter(Mandatory = $true)][string]$Out,
  [int]$TimeoutSec = 300
)
$ErrorActionPreference = 'Stop'
$source = (Resolve-Path -LiteralPath $Project).Path
$Out = [IO.Path]::GetFullPath($Out)

function Get-Prop($obj, $name) {
  try { [System.__ComObject].InvokeMember($name, [Reflection.BindingFlags]::GetProperty, $null, $obj, $null) } catch { 'ERR' }
}

$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(60)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 60 minutes.' }
$o = $null
$started = @()
$lines = New-Object System.Collections.Generic.List[string]
try {
  $tmp = Join-Path ([IO.Path]::GetTempPath()) ('opjfeat-' + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Path $tmp | Out-Null
  $copy = Join-Path $tmp ([IO.Path]::GetFileName($source))
  Copy-Item -LiteralPath $source -Destination $copy
  $before = @(Get-Process | Where-Object { $_.Name -like 'Origin*' -or $_.Name -like 'ORIGIN~*' } | ForEach-Object { $_.Id })
  $o = New-Object -ComObject Origin.Application
  $started = @(Get-Process | Where-Object { ($_.Name -like 'Origin*' -or $_.Name -like 'ORIGIN~*') -and $before -notcontains $_.Id } | ForEach-Object { $_.Id })
  Add-Type -TypeDefinition 'using System; using System.Diagnostics; using System.Threading; public static class OpjWatchdog4 { static Timer t; public static void Start(int[] pids, int ms) { t = new Timer(delegate { foreach (int p in pids) { try { Process.GetProcessById(p).Kill(); } catch { } } }, null, ms, Timeout.Infinite); } public static void Stop() { if (t != null) t.Dispose(); } }' -ErrorAction SilentlyContinue
  [OpjWatchdog4]::Start([int[]]$started, $TimeoutSec * 1000)
  $ok = $o.Load($copy, $true)
  $lines.Add("# loaded=$ok")
  $books = New-Object System.Collections.Generic.List[string]
  function Walk($folder) {
    $lines.Add("FOLDER`t$(Get-Prop $folder 'Path')`t$(Get-Prop $folder 'Name')")
    foreach ($p in (Get-Prop $folder 'PageBases')) {
      $name = Get-Prop $p 'Name'
      $type = Get-Prop $p 'Type'
      $lines.Add("PAGE`t$(Get-Prop $folder 'Path')`t$name`t$type")
      if ($type -eq 2) { $books.Add($name) }
    }
    foreach ($s in (Get-Prop $folder 'Folders')) { Walk $s }
  }
  Walk (Get-Prop $o 'RootFolder')
  foreach ($book in $books) {
    [void]$o.Execute("win -a $book; double nl = page.nlayers;")
    $layers = [int]$o.LTVar('nl')
    for ($s = 1; $s -le $layers; $s++) {
      [void]$o.Execute("win -a $book; page.active = $s; double nc = wks.ncols;")
      $cols = [int]$o.LTVar('nc')
      for ($c = 1; $c -le $cols; $c++) {
        [void]$o.Execute("string fx`$ = wks.col$c.formula`$; string cn`$ = wks.col$c.name`$; double rm = wks.col$c.svrm;")
        $fx = [string]$o.LTStr('fx$')
        $rm = [int]$o.LTVar('rm')
        if ($fx.Length -gt 0 -or $rm -ne 0) { $lines.Add("COL`t$book`t$s`t$c`t$($o.LTStr('cn$'))`t$rm`t$fx") }
      }
    }
  }
}
catch { $lines.Add("# ERROR $_") }
finally {
  try { [OpjWatchdog4]::Stop() } catch { }
  if ($null -ne $o) {
    try { [void]$o.Exit() } catch { $lines.Add("# warning: Origin did not exit cleanly: $_") }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($o)
    Start-Sleep -Seconds 2
  }
  foreach ($p in $started) { try { $pr = Get-Process -Id $p -ErrorAction Stop; if (-not $pr.HasExited) { $pr.Kill() } } catch { } }
  [IO.File]::WriteAllLines($Out, $lines, (New-Object Text.UTF8Encoding($false)))
  $lock.ReleaseMutex()
  $lock.Dispose()
}

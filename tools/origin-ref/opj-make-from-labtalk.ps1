<#
.SYNOPSIS
  Runs a LabTalk script (one command per line) in a hidden Origin, saves the project as an .opj
  file and exports every graph window as a PNG. Used to see how Origin writes a graph that we
  build with chosen settings.

.DESCRIPTION
  Lines starting with '#' and empty lines are skipped. A line is run with Origin.Execute; a line
  that Origin refuses is reported as "ERR line N" on the console and the run goes on. The project
  is saved to -Out (an existing file is never overwritten). The PNG of each graph goes to
  -PngDir as <graph name>.png (800 pixels wide). Takes the system-wide lock
  Global\SheafOriginReference, is killed by a watchdog after -TimeoutSec seconds, and closes only
  the Origin it started.

.EXAMPLE
  .\opj-make-from-labtalk.ps1 -Script ungroup.lt.txt -Out C:\scratch\ungroup.opj -PngDir C:\scratch\png
#>
param(
  [Parameter(Mandatory = $true)][string]$Script,
  [Parameter(Mandatory = $true)][string]$Out,
  [Parameter(Mandatory = $true)][string]$PngDir,
  [int]$TimeoutSec = 300
)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $Out) { throw "Refusing to overwrite $Out" }
$Out = [IO.Path]::GetFullPath($Out)
New-Item -ItemType Directory -Force -Path $PngDir | Out-Null
$PngDir = (Resolve-Path -LiteralPath $PngDir).Path
$lines = Get-Content -LiteralPath $Script

$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(60)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 60 minutes.' }
$o = $null
$started = @()
try {
  $before = @(Get-Process | Where-Object { $_.Name -like 'Origin*' -or $_.Name -like 'ORIGIN~*' } | ForEach-Object { $_.Id })
  $o = New-Object -ComObject Origin.Application
  $started = @(Get-Process | Where-Object { ($_.Name -like 'Origin*' -or $_.Name -like 'ORIGIN~*') -and $before -notcontains $_.Id } | ForEach-Object { $_.Id })
  Add-Type -TypeDefinition 'using System; using System.Diagnostics; using System.Threading; public static class OpjWatchdog3 { static Timer t; public static void Start(int[] pids, int ms) { t = new Timer(delegate { foreach (int p in pids) { try { Process.GetProcessById(p).Kill(); } catch { } } }, null, ms, Timeout.Infinite); } public static void Stop() { if (t != null) t.Dispose(); } }' -ErrorAction SilentlyContinue
  [OpjWatchdog3]::Start([int[]]$started, $TimeoutSec * 1000)
  [void]$o.NewProject()
  $n = 0
  foreach ($line in $lines) {
    $n++
    $t = $line.Trim()
    if ($t.Length -eq 0 -or $t.StartsWith('#')) { continue }
    $t = $t.Replace('%PNGDIR%', $PngDir)
    if (-not $o.Execute($t)) { "ERR line $n : $t" }
  }
  # every graph window to a PNG
  [void]$o.Execute('double np = 0; doc -e P { np++; }')
  $count = [int]$o.LTVar('np')
  for ($k = 1; $k -le $count; $k++) {
    [void]$o.Execute("double kk = 0; string wn`$ = `"`"; doc -e P { kk++; if (kk == $k) { wn`$ = page.name`$; } }")
    $name = [string]$o.LTStr('wn$')
    [void]$o.Execute("win -a $name; expGraph type:=png path:=`"$PngDir`" filename:=$name overwrite:=1 tr1.unit:=2 tr1.width:=800;")
  }
  if (-not $o.Save($Out)) { throw "Origin could not save $Out" }
  "saved $Out ($count graph(s))"
}
finally {
  try { [OpjWatchdog3]::Stop() } catch { }
  if ($null -ne $o) {
    try { [void]$o.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($o)
    Start-Sleep -Seconds 2
  }
  foreach ($p in $started) { try { $pr = Get-Process -Id $p -ErrorAction Stop; if (-not $pr.HasExited) { $pr.Kill() } } catch { } }
  $lock.ReleaseMutex()
  $lock.Dispose()
}

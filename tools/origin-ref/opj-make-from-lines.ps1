<#
.SYNOPSIS
  Builds an Origin project from a file of LabTalk lines (one command per line, "#" starts a comment),
  after creating the source workbook GSrc (21 rows, columns A..E of closed-form data), saves it as
  an .opj and exports a PNG of every graph window. Used to see which bytes Origin writes for a
  graph feature (area plots, number formats, plot groups, panels).

.DESCRIPTION
  System-wide lock Global\SheafOriginReference, own hidden Origin through COM, closed at the end.
  The output file is never overwritten. A line "?name" prints the LabTalk number, "$name$" the
  LabTalk string, into -Report.
#>
param(
  [Parameter(Mandatory = $true)][string]$Lines,
  [string]$Out = '',
  [Parameter(Mandatory = $true)][string]$Report,
  [string]$PngDir = '',
  # An existing project to load first (copied to a temporary folder; it is never changed).
  [string]$Open = ''
)
$ErrorActionPreference = 'Stop'
if ($Out -ne '' -and (Test-Path -LiteralPath $Out)) { throw "Refusing to overwrite $Out" }
if ($Out -ne '') { $Out = [IO.Path]::GetFullPath($Out) }
$rep = New-Object IO.StreamWriter($Report, $false, (New-Object Text.UTF8Encoding($false)))
$rep.NewLine = "`n"
$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(60)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'lock timeout' }
$o = $null
try {
  $before = @(Get-Process | Where-Object { $_.Name -like 'Origin*' -or $_.Name -like 'ORIGIN~*' } | ForEach-Object { $_.Id })
  $o = New-Object -ComObject Origin.Application
  $started = @(Get-Process | Where-Object { ($_.Name -like 'Origin*' -or $_.Name -like 'ORIGIN~*') -and $before -notcontains $_.Id } | ForEach-Object { $_.Id })
  Add-Type -TypeDefinition 'using System; using System.Diagnostics; using System.Threading; public static class OpjWd6 { static Timer t; public static void Start(int[] pids, int ms) { t = new Timer(delegate { foreach (int p in pids) { try { Process.GetProcessById(p).Kill(); } catch { } } }, null, ms, Timeout.Infinite); } public static void Stop() { if (t != null) t.Dispose(); } }' -ErrorAction SilentlyContinue
  [OpjWd6]::Start([int[]]$started, 300000)
  function Lt([string]$s) { [void]$o.Execute($s) }
  if ($Open -ne '') {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('opjl-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    $copy = Join-Path $tmp ([IO.Path]::GetFileName($Open))
    Copy-Item -LiteralPath $Open -Destination $copy
    if (-not $o.Load($copy, $true)) { throw "Origin could not load $Open" }
  } else {
    [void]$o.NewProject()
    Lt 'newbook name:=GSrc sheet:=1 option:=lsname;'
    Lt 'win -a GSrc; wks.ncols = 5; wks.nrows = 21;'
    Lt 'loop(r, 1, 21) { x = (r - 1) * 0.5; cell(r, 1) = x; cell(r, 2) = x * x; cell(r, 3) = 100 - x * 5; cell(r, 4) = 10 ^ (x / 5); cell(r, 5) = sin(x) * 5; }'
  }
  foreach ($line in Get-Content -LiteralPath $Lines) {
    $t = $line.Trim()
    if ($t -eq '' -or $t.StartsWith('#')) { continue }
    if ($t.StartsWith('?')) { $rep.WriteLine($t.Substring(1) + "=" + $o.LTVar($t.Substring(1))); continue }
    if ($t.StartsWith('$')) { $rep.WriteLine($t + "=" + $o.LTStr($t)); continue }
    $ok = $o.Execute($t)
    if (-not $ok) { $rep.WriteLine("FAILED: $t") }
  }
  if ($PngDir -ne '') {
    New-Item -ItemType Directory -Force -Path $PngDir | Out-Null
    $PngDir = (Resolve-Path -LiteralPath $PngDir).Path
    Lt 'double ng = 0; doc -e P { ng++; }'
    $n = [int]$o.LTVar('ng')
    for ($k = 1; $k -le $n; $k++) {
      Lt "double kk = 0; string wn`$ = `"`"; doc -e P { kk++; if (kk == $k) { wn`$ = page.name`$; } }"
      $g = [string]$o.LTStr('wn$')
      Lt "win -a $g;"
      Lt ("expGraph type:=png path:=`"$PngDir`" filename:=$g overwrite:=1 tr1.unit:=2 tr1.width:=800;")
    }
  }
  if ($Out -ne '') {
    if (-not $o.Save($Out)) { throw "Origin could not save $Out" }
    "saved $Out"
  }
}
finally {
  $rep.Dispose()
  try { [OpjWd6]::Stop() } catch { }
  if ($null -ne $o) {
    try { [void]$o.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($o)
    Start-Sleep -Seconds 2
  }
  foreach ($p in $started) { try { $pr = Get-Process -Id $p -ErrorAction Stop; if (-not $pr.HasExited) { $pr.Kill() } } catch { } }
  $lock.ReleaseMutex()
  $lock.Dispose()
}

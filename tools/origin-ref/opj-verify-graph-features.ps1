<#
.SYNOPSIS
  Like opj-verify-graphs.ps1, and for every layer also what the written project features come out as
  in Origin: the layer link (layer.link, x.link, y.link, unit), the axis showAxes and showLabels,
  the axis colours, and the PNG of every graph (used for the stack offsets, double-Y, bar spacing and
  axis side features of docs/spec/opj-format.md 13.12).

.DESCRIPTION
  For each file: the file is copied to a temporary folder (the original is only read), loaded in
  its own hidden Origin through COM, and for every graph window (`doc -e P`) the script writes
  "name=value" lines (number of layers, per layer the axis limits, steps and scale types, the
  frame, the number of plots) to <OutDir>\<file>.origin.txt and the graph as <OutDir>\<file>.<graph>.png
  (expGraph, 800 pixels wide). The project is never saved.

  Like the other scripts it takes the system-wide lock Global\SheafOriginReference, is killed by a
  watchdog after -TimeoutSec seconds (a modal dialog in a hidden Origin would otherwise hang it)
  and closes only the Origin instance that it started.

  Output line kinds: "FILE <name> loaded=<True|False>", "GRAPH <name>", "LAYER <n>", "key=value",
  "ERR <text>" when Origin refused a call.

.EXAMPLE
  .\opj-verify-graphs.ps1 -Files a.opj,b.opj -OutDir C:\scratch\opjg\verify
#>
param(
  [Parameter(Mandatory = $true)][string[]]$Files,
  [Parameter(Mandatory = $true)][string]$OutDir,
  [int]$TimeoutSec = 300
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$OutDir = (Resolve-Path -LiteralPath $OutDir).Path
$failed = 0

$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(60)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 60 minutes.' }
try {
  foreach ($file in $Files) {
    $source = (Resolve-Path -LiteralPath $file).Path
    $base = [IO.Path]::GetFileNameWithoutExtension($source)
    $report = Join-Path $OutDir "$base.origin.txt"
    $tempDir = Join-Path ([IO.Path]::GetTempPath()) ('opjgv-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    $opened = Join-Path $tempDir ([IO.Path]::GetFileName($source))
    Copy-Item -LiteralPath $source -Destination $opened
    $w = New-Object IO.StreamWriter($report, $false, (New-Object Text.UTF8Encoding($false)))
    $w.NewLine = "`n"
    $origin = $null
    $started = @()
    try {
      $before = @(Get-Process | Where-Object { $_.Name -like 'Origin*' -or $_.Name -like 'ORIGIN~*' } | ForEach-Object { $_.Id })
      $origin = New-Object -ComObject Origin.Application
      $started = @(Get-Process | Where-Object { ($_.Name -like 'Origin*' -or $_.Name -like 'ORIGIN~*') -and $before -notcontains $_.Id } | ForEach-Object { $_.Id })
      Add-Type -TypeDefinition 'using System; using System.Diagnostics; using System.Threading; public static class OpjWatchdog5 { static Timer t; public static void Start(int[] pids, int ms) { t = new Timer(delegate { foreach (int p in pids) { try { Process.GetProcessById(p).Kill(); } catch { } } }, null, ms, Timeout.Infinite); } public static void Stop() { if (t != null) t.Dispose(); } }' -ErrorAction SilentlyContinue
      [OpjWatchdog5]::Start([int[]]$started, $TimeoutSec * 1000)
      function Lt([string]$s) { return $origin.Execute($s) }
      function Num([string]$name) { return [double]$origin.LTVar($name) }
      $ok = $origin.Load($opened, $true)
      $w.WriteLine("FILE $base loaded=$ok")
      [void](Lt 'double np = 0; doc -e P { np++; }')
      $n = [int](Num 'np')
      $w.WriteLine("graphs=$n")
      $names = @()
      for ($k = 1; $k -le $n; $k++) {
        [void](Lt "double kk = 0; string wn`$ = `"`"; doc -e P { kk++; if (kk == $k) { wn`$ = page.name`$; } }")
        $names += [string]$origin.LTStr('wn$')
      }
      foreach ($g in $names) {
        $w.WriteLine("GRAPH $g")
        try {
          [void](Lt "win -a $g;")
          [void](Lt 'double nl = page.nlayers; string pl$ = page.longname$;')
          $w.WriteLine("nlayers=" + (Num 'nl'))
          $w.WriteLine("longname=" + [string]$origin.LTStr('pl$'))
          [void](Lt 'double pw = page.width; double ph = page.height;')
          $w.WriteLine("page=" + (Num 'pw') + "x" + (Num 'ph'))
          for ($l = 1; $l -le [int](Num 'nl'); $l++) {
            [void](Lt "win -a $g; page.active = $l;")
            $w.WriteLine("LAYER $l")
            [void](Lt 'double xf = layer.x.from; double xt = layer.x.to; double xi = layer.x.inc; double xs = layer.x.type; double xm = layer.x.minorTicks;')
            [void](Lt 'double yf = layer.y.from; double yt = layer.y.to; double yi = layer.y.inc; double ys = layer.y.type; double ym = layer.y.minorTicks;')
            [void](Lt 'double ll = layer.left; double lt = layer.top; double lw = layer.width; double lh = layer.height; double npl = layer.nplots;')
            foreach ($v in 'xf', 'xt', 'xi', 'xs', 'xm', 'yf', 'yt', 'yi', 'ys', 'ym', 'll', 'lt', 'lw', 'lh', 'npl') { $w.WriteLine("$v=" + (Num $v)) }
            [void](Lt 'double lk = layer.link; double xl = layer.x.link; double yl = layer.y.link; double un = layer.unit;')
            [void](Lt 'double xa = layer.x.showAxes; double ya = layer.y.showAxes; double xb = layer.x.showLabels; double yb = layer.y.showLabels; double xc = layer.x.color; double yc = layer.y.color;')
            foreach ($v in 'lk', 'xl', 'yl', 'un', 'xa', 'ya', 'xb', 'yb', 'xc', 'yc') { $w.WriteLine("$v=" + (Num $v)) }
            for ($p = 1; $p -le [int](Num 'npl'); $p++) {
              [void](Lt "layer.plot = $p; string pn`$ = %C; double pt = 0;")
              $w.WriteLine("plot$p=" + [string]$origin.LTStr('pn$'))
            }
          }
          [void](Lt 'layer -a;')
          # The folder and the file name go in apart (a full path in filename makes the export fail).
          $png = "$base.$g"
          $okx = Lt ("expGraph type:=png path:=`"$OutDir`" filename:=`"$png`" overwrite:=1 tr1.unit:=2 tr1.width:=800;")
          $w.WriteLine("export=$okx")
        } catch { $w.WriteLine("ERR " + ($_.Exception.Message -replace "[\r\n]+", ' ')) }
      }
      $w.Flush()
    } catch {
      $w.WriteLine("ERR " + ($_.Exception.Message -replace "[\r\n]+", ' '))
      $failed++
    } finally {
      try { [OpjWatchdog5]::Stop() } catch { }
      $w.Dispose()
      if ($null -ne $origin) {
        try { [void]$origin.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)
        Start-Sleep -Seconds 2
      }
      foreach ($p in $started) { try { $pr = Get-Process -Id $p -ErrorAction Stop; if (-not $pr.HasExited) { $pr.Kill() } } catch { } }
      Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    "$base : see $report"
  }
}
finally {
  $lock.ReleaseMutex()
  $lock.Dispose()
}
exit $failed

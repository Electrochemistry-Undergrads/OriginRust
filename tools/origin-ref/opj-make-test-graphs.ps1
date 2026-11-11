<#
.SYNOPSIS
  Builds a small Origin project with graphs whose settings we choose here (axis ranges, scale
  types, plot types, colours, line widths, symbols, titles), saves it as an .opj file and
  exports every graph to a PNG, so the graph records of the file can be read back and checked.

.DESCRIPTION
  Origin is started hidden through COM (system-wide lock, like run-labtalk.ps1). The project
  is saved to -Out (a new file; an existing file is never overwritten). The PNG of each graph
  goes to -PngDir, and the values that Origin reports for each graph (axis limits, scale type,
  position of the layer, plot settings) go to -Report as "name=value" lines, so a reader can be
  tested against what Origin itself says about the same file.

.EXAMPLE
  .\opj-make-test-graphs.ps1 -Out C:\scratch\graphs.opj -PngDir C:\scratch\png -Report C:\scratch\graphs.txt
#>
param(
  [Parameter(Mandatory = $true)][string]$Out,
  [Parameter(Mandatory = $true)][string]$PngDir,
  [Parameter(Mandatory = $true)][string]$Report
)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $Out) { throw "Refusing to overwrite $Out" }
$Out = [IO.Path]::GetFullPath($Out)
New-Item -ItemType Directory -Force -Path $PngDir | Out-Null
$PngDir = (Resolve-Path -LiteralPath $PngDir).Path
$rep = New-Object IO.StreamWriter($Report, $false, (New-Object Text.UTF8Encoding($false)))
$rep.NewLine = "`n"

$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(60)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 60 minutes.' }
$o = $null
try {
  $o = New-Object -ComObject Origin.Application
  function Lt([string]$s) { [void]$o.Execute($s) }
  function Var([string]$name) { return $o.LTVar($name) }
  function Str([string]$name) { return $o.LTStr($name) }
  [void]$o.NewProject()

  # One source workbook with five columns of closed-form data (21 rows).
  Lt 'newbook name:=GSrc sheet:=1 option:=lsname;'
  Lt '[GSrc]1!wks.ncols = 5;'
  Lt '[GSrc]1!col(A) = data(0, 10, 0.5);'
  Lt '[GSrc]1!col(B) = col(A) * col(A);'
  Lt '[GSrc]1!col(C) = 100 - col(A) * 5;'
  Lt '[GSrc]1!col(D) = 10 ^ (col(A) / 5);'
  Lt '[GSrc]1!col(E) = sin(col(A)) * 5;'
  Lt '[GSrc]1!wks.col2.lname$ = Squares; [GSrc]1!wks.col3.lname$ = Line;'

  # name, plot type, columns, then LabTalk that formats it. Plot types: 200 line, 201 scatter,
  # 202 line+symbol, 203 column.
  $graphs = @(
    @{ name = 'GLine'; plot = 200; cols = '(1,2:3)'
       fmt = 'layer.x.from = 0; layer.x.to = 10; layer.x.inc = 2; layer.y.from = 0; layer.y.to = 100; layer.y.inc = 20; label -xb Time (s); label -yl Current (mA);'
       plotfmt = @('set %C -c 2 -w 1000;', 'set %C -c 4 -w 1500;') },
    @{ name = 'GScat'; plot = 201; cols = '(1,4)'
       fmt = 'layer.x.type = 1; layer.x.from = 1; layer.x.to = 10; layer.y.type = 1; layer.y.from = 1; layer.y.to = 100;'
       plotfmt = @('set %C -c 3 -k 2 -z 12;') },
    @{ name = 'GLS'; plot = 202; cols = '(1,5)'
       fmt = 'layer.x.from = 2; layer.x.to = 8; layer.x.inc = 2; layer.y.from = -6; layer.y.to = 6; layer.y.inc = 3; layer.left = 20; layer.top = 15; layer.width = 60; layer.height = 55;'
       plotfmt = @('set %C -c 6 -l 3 -w 2000 -k 3 -z 9;') },
    @{ name = 'GCol'; plot = 203; cols = '(1,2)'
       fmt = 'layer.y.from = 0; layer.y.to = 120; layer.y.inc = 30;'
       plotfmt = @('set %C -c 5;') }
  )
  foreach ($g in $graphs) {
    Lt ("plotxy iy:=[GSrc]1!" + $g.cols + " plot:=" + $g.plot + " ogl:=[<new>];")
    Lt ("win -r %H " + $g.name + ";")
    Lt ("win -a " + $g.name + ";")
    Lt $g.fmt
    $k = 1
    foreach ($pf in $g.plotfmt) { Lt ("layer.plot = $k;"); Lt $pf; $k++ }
    Lt 'layer -a;'
    # what Origin says about the graph
    Lt 'double xf = layer.x.from; double xt = layer.x.to; double xi = layer.x.inc; double xs = layer.x.type;'
    Lt 'double yf = layer.y.from; double yt = layer.y.to; double yi = layer.y.inc; double ys = layer.y.type;'
    Lt 'double ll = layer.left; double lt = layer.top; double lw = layer.width; double lh = layer.height; double np = layer.nplots;'
    $rep.WriteLine("GRAPH " + $g.name)
    foreach ($v in 'xf', 'xt', 'xi', 'xs', 'yf', 'yt', 'yi', 'ys', 'll', 'lt', 'lw', 'lh', 'np') { $rep.WriteLine("$v=" + (Var $v)) }
    $png = Join-Path $PngDir ($g.name + '.png')
    Lt ("expGraph type:=png filename:=`"$png`" overwrite:=1 tr1.unit:=2 tr1.width:=800;")
  }
  if (-not $o.Save($Out)) { throw "Origin could not save $Out" }
  "saved $Out"
}
finally {
  $rep.Dispose()
  if ($null -ne $o) {
    try { [void]$o.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($o)
    Start-Sleep -Seconds 2
  }
  $lock.ReleaseMutex()
  $lock.Dispose()
}

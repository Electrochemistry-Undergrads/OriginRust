<#
.SYNOPSIS
  Lists the windows of one or more Origin projects as Origin itself sees them: the names of the
  workbooks, graphs and matrices of each project.

.DESCRIPTION
  Each project is copied to a temporary folder (the original is only read), opened read-only in
  its own hidden Origin instance through COM, and the windows are counted with LabTalk's
  `doc -e W`, `doc -e P` and `doc -e M` loops (workbooks, graphs, matrices; `doc -e G` loops over
  graph layers and is not used). Output, one line per
  window, tab separated:

    <project file name>  <W | P | M>  <window name>

  It is used to check which windows our .opj reader should call graphs (see section 10 of
  docs/spec/opj-format.md). The projects are never saved. Like run-labtalk.ps1 this takes the
  system-wide lock Global\SheafOriginReference, and Origin is closed in a finally block.

.EXAMPLE
  .\opj-list-windows.ps1 -Projects "C:\data\a.opj","C:\data\b.opj" -Out windows.txt
#>
param(
  [Parameter(Mandatory = $true)][string[]]$Projects,
  [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Stop'

$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 30 minutes.' }
$w = New-Object IO.StreamWriter($Out, $false, (New-Object Text.UTF8Encoding($false)))
$w.NewLine = "`n"
try {
  foreach ($project in $Projects) {
    $source = (Resolve-Path -LiteralPath $project).Path
    $tempDir = Join-Path ([IO.Path]::GetTempPath()) ('opjlist-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    $opened = Join-Path $tempDir ([IO.Path]::GetFileName($source))
    Copy-Item -LiteralPath $source -Destination $opened
    $origin = $null
    $label = [IO.Path]::GetFileName($source)
    try {
      $origin = New-Object -ComObject Origin.Application
      $ok = $origin.Load($opened, $true)
      $w.WriteLine("# $label loaded=$ok")
      foreach ($kind in 'W', 'P', 'M') {
        [void]$origin.Execute("double nn = 0; doc -e $kind { nn++; }")
        $n = [int]$origin.LTVar('nn')
        for ($k = 1; $k -le $n; $k++) {
          [void]$origin.Execute("double kk = 0; string wn`$ = `"`"; doc -e $kind { kk++; if (kk == $k) { wn`$ = page.name`$; } }")
          $w.WriteLine("$label`t$kind`t" + [string]$origin.LTStr('wn$'))
        }
      }
      $w.Flush()
    }
    catch { $w.WriteLine("# $label ERROR $_") }
    finally {
      if ($null -ne $origin) {
        try { [void]$origin.Exit() } catch { $w.WriteLine("# warning: Origin did not exit cleanly: $_") }
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)
        Start-Sleep -Seconds 2
      }
      Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
  }
}
finally {
  $w.Dispose()
  $lock.ReleaseMutex()
  $lock.Dispose()
}

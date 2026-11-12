<#
.SYNOPSIS
  Runs a LabTalk script made of chunks in a hidden Origin through COM, then prints the values of
  chosen variables and the cells of chosen worksheets (full precision, tab separated).

.DESCRIPTION
  Like run-labtalk.ps1 and run-labtalk-data.ps1, with two differences: the script is split at
  lines that begin with ### and each chunk is executed on its own (a failing chunk prints
  "chunk=False" and the next chunk still runs), and any number of worksheets can be dumped
  (-Sheets takes names such as '[Book1]Sheet1' and report sheets such as '[Book1]FitNL1').
  Variables that were never set read back as -1.23456789E-300.
  Takes the same system-wide lock as the other scripts; Origin is closed in the finally block.
#>
param(
  [Parameter(Mandatory = $true)][string]$Script,
  [string[]]$Vars = @(),
  [string[]]$StrVars = @(),
  [string[]]$Sheets = @('[Book1]Sheet1')
)
$ErrorActionPreference = 'Stop'
$inv = [Globalization.CultureInfo]::InvariantCulture
$text = Get-Content -Raw -LiteralPath $Script
$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'lock timeout' }
$origin = $null
try {
  $origin = New-Object -ComObject Origin.Application
  $ok = $true
  $nl = [Environment]::NewLine
  foreach ($chunk in ($text -split '(?m)^###[^\r\n]*\r?\n')) { if ($chunk.Trim()) { $r1 = $origin.Execute($chunk); "chunk=$r1" } }
  "execute=$ok"
  foreach ($v in $Vars) { $value = [double]$origin.LTVar($v); "$v=" + $value.ToString('R', $inv) }
  foreach ($v in $StrVars) { "$v=" + $origin.LTStr($v) }
  $t = [System.__ComObject]
  foreach ($sh in $Sheets) {
    $r = $null
    try { $r = $t.InvokeMember('GetWorksheet', [Reflection.BindingFlags]::InvokeMethod, $null, $origin, @($sh)) } catch { "#ERR $sh $_" }
    if ($r -is [Array]) {
      $rows = $r.GetLength(0); $cols = $r.GetLength(1)
      "#SHEET $sh $rows $cols"
      $l0 = $r.GetLowerBound(0); $l1 = $r.GetLowerBound(1)
      for ($i = $l0; $i -lt $l0 + $rows; $i++) {
        $line = for ($j = $l1; $j -lt $l1 + $cols; $j++) { $c = $r[$i, $j]; if ($c -is [DBNull] -or $null -eq $c) { '' } elseif ($c -is [double]) { $c.ToString('R', $inv) } else { [string]$c } }
        $line -join "`t"
      }
    }
  }
}
finally {
  if ($null -ne $origin) {
    try { [void]$origin.Exit() } catch { "warning: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)
    Start-Sleep -Seconds 2
  }
  $lock.ReleaseMutex(); $lock.Dispose()
}

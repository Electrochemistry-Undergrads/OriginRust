<#
.SYNOPSIS
  Like run-labtalk.ps1, but also prints the whole first worksheet of the first
  workbook ("Book1") as CSV with full precision, so scripts can return whole
  result columns, not just scalar variables.

.DESCRIPTION
  Output format: "execute=True", then "name=value" lines for each -Vars name,
  then a line "#SHEET rows cols" followed by one CSV line per row.
  A script that wants its results read back puts them into columns of the
  workbook it creates with "newbook;" (it must be the first workbook).

.EXAMPLE
  .\run-labtalk-data.ps1 -Script smooth-suite.labtalk.txt -Vars a,b
#>
param(
  [Parameter(Mandatory = $true)][string]$Script,
  [string[]]$Vars = @(),
  [string]$Sheet = '[Book1]Sheet1'
)
$ErrorActionPreference = 'Stop'
$inv = [Globalization.CultureInfo]::InvariantCulture
$text = Get-Content -Raw -LiteralPath $Script
# Only one Origin instance may run at a time. A named lock makes a second copy of this
# script wait (up to 30 minutes) until the first one has closed Origin.
$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 30 minutes.' }
$origin = $null
try {
  $origin = New-Object -ComObject Origin.Application
  $ok = $origin.Execute($text)
  "execute=$ok"
  foreach ($v in $Vars) {
    $value = [double]$origin.LTVar($v)
    "$v=" + $value.ToString('R', $inv)
  }
  $t = [System.__ComObject]
  $r = $t.InvokeMember('GetWorksheet', [Reflection.BindingFlags]::InvokeMethod, $null, $origin, @($Sheet))
  if ($r -is [Array]) {
    $rows = $r.GetLength(0); $cols = $r.GetLength(1)
    "#SHEET $rows $cols"
    $l0 = $r.GetLowerBound(0); $l1 = $r.GetLowerBound(1)
    for ($i = $l0; $i -lt $l0 + $rows; $i++) {
      $line = for ($j = $l1; $j -lt $l1 + $cols; $j++) { $c = $r[$i, $j]; if ($c -is [DBNull] -or $null -eq $c) { 'NaN' } else { ([double]$c).ToString('R', $inv) } }
      $line -join ','
    }
  }
}
finally {
  if ($null -ne $origin) {
    try { [void]$origin.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)
    # Give the Origin process a moment to end before the next run starts one.
    Start-Sleep -Seconds 2
  }
  $lock.ReleaseMutex()
  $lock.Dispose()
}
